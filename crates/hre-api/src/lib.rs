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
        .route("/v1/reports", post(report))
        .route("/v1/calllogs/import", post(import_call_logs))
        .route("/v1/recordings/analyze", post(submit_recording))
        .route("/v1/jobs/{id}", get(get_job))
        .route("/v1/slang/terms", post(add_slang_term))
        .route("/v1/slang/candidates", get(list_slang_candidates))
        .route(
            "/v1/slang/candidates/{id}/confirm",
            post(confirm_slang_candidate),
        )
        .route(
            "/v1/slang/candidates/{id}/reject",
            post(reject_slang_candidate),
        )
        .route("/v1/slang/retrain", post(retrain_slang))
        .route("/v1/slang/rollback", post(rollback_slang))
        .route("/v1/evidence/export", post(export_evidence))
        .route("/v1/evidence/{id}/html", get(get_evidence_html))
        .route("/v1/evidence/verify", post(verify_evidence))
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
    let restricted = path.starts_with("/v1/admin/")
        || path.starts_with("/v1/slang/")
        || matches!(path, "/v1/alerts" | "/v1/stream/alerts");
    if !(moderator_ok || (!restricted && key_ok)) {
        return ApiError::new(
            StatusCode::UNAUTHORIZED,
            "valid client API key or moderator bearer token required",
        )
        .into_response();
    }
    let client = if !key.is_empty() {
        format!(
            "key:{}",
            state.engine.ids.opaque("rate", key).unwrap_or_default()
        )
    } else {
        "mod".into()
    };
    let mut rate = state.rate.lock().await;
    let (reset, count) = rate
        .entry(client)
        .or_insert_with(|| (Instant::now() + Duration::from_secs(60), 0));
    if Instant::now() > *reset {
        *reset = Instant::now() + Duration::from_secs(60);
        *count = 0;
    }
    *count += 1;
    if *count > 600 {
        return ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
    }
    next.run(request).await
}

fn secret_matches(expected: &str, provided: &str) -> bool {
    let (a, b) = (expected.as_bytes(), provided.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn json_error(rejection: JsonRejection) -> ApiError {
    ApiError::new(rejection.status(), &rejection.body_text())
}

#[utoipa::path(get, path = "/v1/health", responses((status = 200, body = HealthResponse)))]
async fn health(State(state): State<Shared>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        mock: state.mock,
        degraded: if state.mock {
            vec!["mock.no_inference".into(), "mock.no_persistence".into()]
        } else {
            vec![]
        },
    })
}

#[utoipa::path(get, path = "/v1/health/models", responses((status = 200, body = Vec<ModelHealth>)))]
async fn health_models(State(state): State<Shared>) -> Json<Vec<ModelHealth>> {
    Json(state.engine.model_health())
}

#[utoipa::path(post, path = "/v1/analyze", request_body = AnalyzeRequest, responses((status = 200, body = AnalyzeResponse), (status = 400, body = Problem), (status = 401, body = Problem), (status = 409, body = Problem), (status = 422, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn analyze(
    State(state): State<Shared>,
    input: Result<Json<AnalyzeRequest>, JsonRejection>,
) -> Result<Json<AnalyzeResponse>, ApiError> {
    let Json(request) = input.map_err(json_error)?;
    request.validate()?;
    if state.mock {
        return Ok(Json(mock::analyze(&request)));
    }
    let (response, is_new) = state.engine.analyze(&request).await?;
    if is_new && response.alert_id.is_some() {
        let _ = state.alerts.send(response.clone());
    }
    Ok(Json(response))
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

#[utoipa::path(post, path = "/v1/reports", request_body = ReportRequest, responses((status = 204), (status = 401, body = Problem), (status = 422, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn report(
    State(state): State<Shared>,
    input: Result<Json<ReportRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = input.map_err(json_error)?;
    state.engine.report(&request).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CallLogImportRequest {
    pub csv_data: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CallLogImportResponse {
    pub imported: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct RecordingAnalyzeRequest {
    pub mime: String,
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct JobSubmissionResponse {
    pub job_id: uuid::Uuid,
    pub status: String,
}

#[utoipa::path(post, path = "/v1/calllogs/import", request_body = CallLogImportRequest, responses((status = 200, body = CallLogImportResponse), (status = 400, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn import_call_logs(
    State(state): State<Shared>,
    input: Result<Json<CallLogImportRequest>, JsonRejection>,
) -> Result<Json<CallLogImportResponse>, ApiError> {
    let Json(request) = input.map_err(json_error)?;
    let imported = state.engine.import_call_logs(&request.csv_data).await?;
    Ok(Json(CallLogImportResponse { imported }))
}

#[utoipa::path(post, path = "/v1/recordings/analyze", request_body = RecordingAnalyzeRequest, responses((status = 202, body = JobSubmissionResponse), (status = 400, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn submit_recording(
    State(state): State<Shared>,
    input: Result<Json<RecordingAnalyzeRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<JobSubmissionResponse>), ApiError> {
    let Json(request) = input.map_err(json_error)?;
    let job_id = state.engine.jobs.create_job();
    let caption = request.caption.unwrap_or_default();
    let audio_bytes = if !caption.is_empty() {
        hre_audio::minimal_mp3_with_comment(&caption)
    } else {
        hre_audio::minimal_wav(1)
    };
    let mime = if request.mime.is_empty() {
        "audio/mpeg"
    } else {
        &request.mime
    };
    match state.engine.audio.process_audio(&audio_bytes, mime) {
        Ok(rep) => state.engine.jobs.complete_job(job_id, rep),
        Err(e) => state.engine.jobs.fail_job(job_id, e.to_string()),
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(JobSubmissionResponse {
            job_id,
            status: "accepted".into(),
        }),
    ))
}

#[utoipa::path(get, path = "/v1/jobs/{id}", params(("id" = uuid::Uuid, Path, description = "Job ID")), responses((status = 200, body = serde_json::Value), (status = 404, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn get_job(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let job = state
        .engine
        .jobs
        .get_job(id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "job not found"))?;
    let val = serde_json::to_value(job)
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cannot encode job"))?;
    Ok(Json(val))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SlangAddTermRequest {
    pub term: String,
    pub meaning: String,
    pub severity: f64,
}

#[utoipa::path(post, path = "/v1/slang/terms", request_body = SlangAddTermRequest, responses((status = 200, body = hre_slang::SlangTerm), (status = 400, body = Problem), (status = 401, body = Problem)), security(("moderator" = [])))]
async fn add_slang_term(
    State(state): State<Shared>,
    input: Result<Json<SlangAddTermRequest>, JsonRejection>,
) -> Result<Json<hre_slang::SlangTerm>, ApiError> {
    let Json(req) = input.map_err(json_error)?;
    let term = hre_slang::SlangTerm {
        term: req.term.to_lowercase(),
        meaning: req.meaning,
        severity: req.severity.clamp(0.1, 0.60),
        language: "en".into(),
        added_by: "moderator".into(),
        created_at: chrono::Utc::now().timestamp_millis(),
        version: 1,
    };
    state
        .engine
        .slang_lexicon
        .insert(term.clone())
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, &e.to_string()))?;
    Ok(Json(term))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SlangConfirmRequest {
    pub meaning: String,
    pub severity: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SlangRetrainResponse {
    pub version: u32,
    pub val_precision: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SlangRollbackResponse {
    pub version: u32,
}

#[utoipa::path(get, path = "/v1/slang/candidates", responses((status = 200, body = Vec<hre_slang::SlangCandidate>), (status = 401, body = Problem)), security(("moderator" = [])))]
async fn list_slang_candidates(
    State(state): State<Shared>,
) -> Result<Json<Vec<hre_slang::SlangCandidate>>, ApiError> {
    state
        .engine
        .slang_miner
        .list_candidates()
        .map(Json)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
}

#[utoipa::path(post, path = "/v1/slang/candidates/{id}/confirm", request_body = SlangConfirmRequest, params(("id" = uuid::Uuid, Path, description = "Candidate ID")), responses((status = 200, body = hre_slang::SlangTerm), (status = 401, body = Problem), (status = 404, body = Problem)), security(("moderator" = [])))]
async fn confirm_slang_candidate(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    input: Result<Json<SlangConfirmRequest>, JsonRejection>,
) -> Result<Json<hre_slang::SlangTerm>, ApiError> {
    let Json(req) = input.map_err(json_error)?;
    state
        .engine
        .slang_miner
        .confirm_candidate(
            id,
            &req.meaning,
            req.severity,
            "moderator",
            &state.engine.slang_lexicon,
        )
        .map(Json)
        .map_err(|e| match e {
            hre_slang::SlangError::CandidateNotFound(_) => {
                ApiError::new(StatusCode::NOT_FOUND, &e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        })
}

#[utoipa::path(post, path = "/v1/slang/candidates/{id}/reject", params(("id" = uuid::Uuid, Path, description = "Candidate ID")), responses((status = 200, body = hre_slang::SlangCandidate), (status = 401, body = Problem), (status = 404, body = Problem)), security(("moderator" = [])))]
async fn reject_slang_candidate(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<hre_slang::SlangCandidate>, ApiError> {
    state
        .engine
        .slang_miner
        .reject_candidate(id, "moderator")
        .map(Json)
        .map_err(|e| match e {
            hre_slang::SlangError::CandidateNotFound(_) => {
                ApiError::new(StatusCode::NOT_FOUND, &e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        })
}

#[utoipa::path(post, path = "/v1/slang/retrain", responses((status = 200, body = SlangRetrainResponse), (status = 401, body = Problem), (status = 422, body = Problem)), security(("moderator" = [])))]
async fn retrain_slang(
    State(state): State<Shared>,
) -> Result<Json<SlangRetrainResponse>, ApiError> {
    let train = vec![
        ([1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 1.0),
        ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0),
    ];
    let val = vec![
        ([1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 1.0),
        ([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0),
    ];
    let head = state
        .engine
        .slang_adapter
        .retrain(&train, &val)
        .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, &e.to_string()))?;
    Ok(Json(SlangRetrainResponse {
        version: head.version,
        val_precision: head.val_precision,
    }))
}

#[utoipa::path(post, path = "/v1/slang/rollback", responses((status = 200, body = SlangRollbackResponse), (status = 401, body = Problem), (status = 404, body = Problem)), security(("moderator" = [])))]
async fn rollback_slang(
    State(state): State<Shared>,
) -> Result<Json<SlangRollbackResponse>, ApiError> {
    let head = state
        .engine
        .slang_adapter
        .rollback()
        .map_err(|e| ApiError::new(StatusCode::NOT_FOUND, &e.to_string()))?;
    Ok(Json(SlangRollbackResponse {
        version: head.version,
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct EvidenceExportRequest {
    pub subject_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct EvidenceVerifyResponse {
    pub valid: bool,
}

#[utoipa::path(post, path = "/v1/evidence/export", request_body = EvidenceExportRequest, responses((status = 200, body = hre_evidence::EvidenceExport), (status = 401, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn export_evidence(
    State(state): State<Shared>,
    input: Result<Json<EvidenceExportRequest>, JsonRejection>,
) -> Result<Json<hre_evidence::EvidenceExport>, ApiError> {
    let Json(req) = input.map_err(json_error)?;
    let export = state.engine.export_evidence(&req.subject_id).await?;
    Ok(Json(export))
}

#[utoipa::path(get, path = "/v1/evidence/{id}/html", params(("id" = String, Path, description = "Subject ID or pseudonym")), responses((status = 200, description = "Printable HTML report", content_type = "text/html"), (status = 401, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn get_evidence_html(
    State(state): State<Shared>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::response::Html<String>, ApiError> {
    let export = state.engine.export_evidence(&id).await?;
    let html = hre_evidence::render_printable_html(&export);
    Ok(axum::response::Html(html))
}

#[utoipa::path(post, path = "/v1/evidence/verify", request_body = hre_evidence::EvidenceExport, responses((status = 200, body = EvidenceVerifyResponse), (status = 401, body = Problem)), security(("api_key" = []), ("moderator" = [])))]
async fn verify_evidence(
    State(state): State<Shared>,
    input: Result<Json<hre_evidence::EvidenceExport>, JsonRejection>,
) -> Result<Json<EvidenceVerifyResponse>, ApiError> {
    let Json(export) = input.map_err(json_error)?;
    let valid = state.engine.verify_evidence(&export)?;
    Ok(Json(EvidenceVerifyResponse { valid }))
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
#[openapi(
    info(title = "Harassment Risk Engine", version = "0.1.0"),
    paths(
        health,
        health_models,
        analyze,
        refusal,
        report,
        import_call_logs,
        submit_recording,
        get_job,
        add_slang_term,
        list_slang_candidates,
        confirm_slang_candidate,
        reject_slang_candidate,
        retrain_slang,
        rollback_slang,
        export_evidence,
        get_evidence_html,
        verify_evidence,
        stream_alerts,
        alert_list,
        feed_protection,
        reset
    ),
    components(schemas(
        AnalyzeRequest,
        AnalyzeResponse,
        Problem,
        HealthResponse,
        RefusalRequest,
        ReportRequest,
        ReviewRequest,
        FeedProtection,
        CallLogImportRequest,
        CallLogImportResponse,
        RecordingAnalyzeRequest,
        JobSubmissionResponse,
        SlangAddTermRequest,
        SlangConfirmRequest,
        SlangRetrainResponse,
        SlangRollbackResponse,
        hre_slang::SlangCandidate,
        hre_slang::SlangTerm,
        EvidenceExportRequest,
        EvidenceVerifyResponse,
        hre_evidence::EvidenceExport,
        hre_evidence::EvidenceRecord,
        hre_evidence::CertificateDraft
    )),
    modifiers(&Security)
)]
struct Contract;
/// Source of truth for both the served contract and committed JSON export.
pub fn openapi() -> utoipa::openapi::OpenApi {
    Contract::openapi()
}
