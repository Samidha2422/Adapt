//! HTTP boundary: authentication, validation, draft isolation and generated OpenAPI.
mod engine;
pub mod mock;

use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use hre_core::*;
use std::{
    collections::BTreeMap,
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, Mutex};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::{AllowOrigin, CorsLayer};
use utoipa::{Modify, OpenApi};

struct AppState {
    config: EngineConfig,
    mock: bool,
    engine: engine::Engine,
    alerts: broadcast::Sender<AnalyzeResponse>,
    rate: Mutex<BTreeMap<String, (Instant, u32)>>,
}
type Shared = Arc<AppState>;

/// Builds a router without binding a socket. Mock mode never opens the database.
pub async fn router(config: EngineConfig, mock: bool) -> anyhow::Result<Router> {
    config.validate()?;
    let engine = engine::Engine::new(&config, mock).await?;
    let (alerts, _) = broadcast::channel(256);
    let origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .map(|origin| origin.parse())
        .collect::<Result<_, _>>()?;
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-api-key"),
        ]);
    let state = Arc::new(AppState {
        config,
        mock,
        engine,
        alerts,
        rate: Mutex::new(BTreeMap::new()),
    });
    Ok(Router::new()
        .route("/v1/health", get(health))
        .route("/v1/health/models", get(health_models))
        .route("/v1/analyze", post(analyze))
        .route("/v1/events/refusal", post(refusal))
        .route("/v1/stream/alerts", get(stream_alerts))
        .route("/v1/alerts", get(alert_list))
        .route("/v1/subjects/{id}/feed-protection", get(feed_protection))
        .route("/v1/admin/demo/reset", post(reset))
        .route("/openapi.json", get(schema))
        .route("/docs", get(docs))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), authorize))
        .layer(cors)
        .with_state(state))
}

struct ApiError(Problem);
impl ApiError {
    fn new(status: StatusCode, detail: &str) -> Self {
        Self(Problem {
            kind: format!("urn:hre:problem:{}", status.as_u16()),
            title: status.canonical_reason().unwrap_or("Error").into(),
            status: status.as_u16(),
            detail: detail.into(),
            instance: None,
        })
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (
            status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            Json(self.0),
        )
            .into_response()
    }
}
impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string())
    }
}
impl From<hre_store::StoreError> for ApiError {
    fn from(error: hre_store::StoreError) -> Self {
        match error {
            hre_store::StoreError::IdempotencyConflict => Self::new(
                StatusCode::CONFLICT,
                "event_id already exists with different content",
            ),
            _ => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage operation failed",
            ),
        }
    }
}

async fn authorize(State(state): State<Shared>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if matches!(path, "/v1/health" | "/openapi.json" | "/docs") {
        return next.run(request).await;
    }
    let moderator = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    let moderator_ok = state
        .config
        .moderator_tokens
        .iter()
        .fold(false, |found, token| {
            secret_matches(token, moderator.unwrap_or("")) | found
        });
    let key = request
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let key_ok = state
        .config
        .api_keys
        .iter()
        .fold(false, |found, token| secret_matches(token, key) | found);
    let restricted =
        path.starts_with("/v1/admin/") || matches!(path, "/v1/alerts" | "/v1/stream/alerts");
    if !(moderator_ok || (!restricted && key_ok)) {
        return ApiError::new(
            StatusCode::UNAUTHORIZED,
            "valid client API key or moderator bearer token required",
        )
        .into_response();
    }
    // Credentials have a bounded configured cardinality; attacker strings never become keys.
    let bucket = if moderator_ok {
        moderator.unwrap_or("")
    } else {
        key
    };
    let mut rate = state.rate.lock().await;
    let entry = rate.entry(bucket.into()).or_insert((Instant::now(), 0));
    if entry.0.elapsed() >= Duration::from_secs(60) {
        *entry = (Instant::now(), 0);
    }
    if entry.1 >= 600 {
        return ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "600 requests per minute per credential",
        )
        .into_response();
    }
    entry.1 += 1;
    drop(rate);
    next.run(request).await
}

#[utoipa::path(get, path = "/v1/health", responses((status = 200, body = HealthResponse)))]
async fn health(State(state): State<Shared>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        mock: state.mock,
        degraded: if state.mock {
            vec!["mock.no_inference".into()]
        } else {
            vec!["text.onnx_missing".into()]
        },
    })
}
#[utoipa::path(post, path = "/v1/analyze", request_body = AnalyzeRequest,
    responses((status = 200, body = AnalyzeResponse), (status = 401, body = Problem, content_type = "application/problem+json"), (status = 409, body = Problem), (status = 413, body = Problem), (status = 422, body = Problem), (status = 429, body = Problem)),
    security(("api_key" = []), ("moderator" = [])))]
async fn analyze(
    State(state): State<Shared>,
    input: Result<Json<AnalyzeRequest>, JsonRejection>,
) -> Result<Json<AnalyzeResponse>, ApiError> {
    let Json(request) = input.map_err(json_error)?;
    request.validate()?;
    let (response, fresh) = if state.mock {
        (mock::analyze(&request), false)
    } else {
        state.engine.analyze(&request).await?
    };
    if fresh && response.alert_id.is_some() {
        let _ = state.alerts.send(response.clone());
    }
    Ok(Json(response))
}
fn json_error(error: JsonRejection) -> ApiError {
    ApiError::new(
        error.status(),
        "invalid JSON envelope or request exceeds body limit",
    )
}
#[utoipa::path(get, path = "/v1/health/models", responses((status = 200, body = Vec<ModelHealth>), (status = 401, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn health_models(State(state): State<Shared>) -> Json<Vec<ModelHealth>> {
    Json(state.engine.model_health())
}
#[utoipa::path(post, path = "/v1/events/refusal", request_body = RefusalRequest, responses((status = 204), (status = 401, body = Problem), (status = 422, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn refusal(
    State(state): State<Shared>,
    input: Result<Json<RefusalRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = input.map_err(json_error)?;
    state.engine.refusal(&request).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path = "/v1/stream/alerts", responses((status = 200, description = "SSE alert and resync events", content_type = "text/event-stream"), (status = 401, body = Problem)), security(("moderator" = [])))]
async fn stream_alerts(
    State(state): State<Shared>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream =
        BroadcastStream::new(state.alerts.subscribe()).filter_map(|message| match message {
            Ok(alert) => serde_json::to_string(&alert)
                .ok()
                .map(|data| Ok(Event::default().event("alert").data(data))),
            Err(_) => Some(Ok(Event::default()
                .event("resync")
                .data("reload /v1/alerts"))),
        });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
#[utoipa::path(get, path = "/v1/alerts", responses((status = 200, body = Vec<AnalyzeResponse>), (status = 401, body = Problem)), security(("moderator" = [])))]
async fn alert_list(State(state): State<Shared>) -> Result<Json<Vec<AnalyzeResponse>>, ApiError> {
    Ok(Json(state.engine.alerts().await?))
}
#[utoipa::path(get, path = "/v1/subjects/{id}/feed-protection", params(("id" = String, Path, description = "HMAC subject ID")), responses((status = 200, body = FeedProtection), (status = 401, body = Problem), (status = 422, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn feed_protection(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<FeedProtection>, ApiError> {
    let enabled = state.engine.feed_protection(&id).await?;
    Ok(Json(FeedProtection {
        subject_id: id,
        enabled,
        actions: if enabled {
            vec![SecondaryAction::CollapseReplies, SecondaryAction::RateLimit]
        } else {
            vec![]
        },
    }))
}
#[utoipa::path(post, path = "/v1/admin/demo/reset", responses((status = 403, body = Problem), (status = 501, body = Problem)), security(("moderator" = [])))]
async fn reset(State(state): State<Shared>) -> Result<StatusCode, ApiError> {
    if !state.config.demo_mode {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "demo_mode must be enabled",
        ));
    }
    Err(ApiError::new(
        StatusCode::NOT_IMPLEMENTED,
        "reset is not implemented; use a fresh demo database",
    ))
}
async fn schema() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi())
}
async fn docs() -> axum::response::Html<&'static str> {
    axum::response::Html("<!doctype html><html lang=en><meta charset=utf-8><title>Harassment Risk Engine API</title><main><h1>API</h1><a href=/openapi.json>OpenAPI JSON</a></main></html>")
}
async fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "route not implemented")
}
async fn method_not_allowed() -> ApiError {
    ApiError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "method not supported for route",
    )
}

struct Security;
impl Modify for Security {
    fn modify(&self, schema: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{
            ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme,
        };
        if let Some(components) = schema.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
            components.add_security_scheme(
                "moderator",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
#[derive(OpenApi)]
#[openapi(info(title = "Harassment Risk Engine", version = "0.1.0"), paths(health, health_models, analyze, refusal, stream_alerts, alert_list, feed_protection, reset),
    components(schemas(AnalyzeRequest, AnalyzeResponse, Problem, HealthResponse, RefusalRequest, ReportRequest, ReviewRequest, FeedProtection)), modifiers(&Security))]
struct Contract;
/// Source of truth for both the served contract and committed JSON export.
pub fn openapi() -> utoipa::openapi::OpenApi {
    Contract::openapi()
}
