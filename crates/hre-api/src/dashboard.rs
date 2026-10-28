//! Moderator browser authentication and the in-process dashboard bridge.
use crate::{secret_matches, AppState, Shared};
use axum::{
    extract::{Form, Request, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use hre_dashboard::backend::{
    DashboardBackend, DashboardFuture, DashboardHealth, DashboardMetrics,
};
use std::time::{Duration, Instant};

const COOKIE: &str = "hre_session";

pub(super) fn session_token(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (name, value) = part.trim().split_once('=')?;
            (name == COOKIE).then(|| value.to_owned())
        })
}

pub(super) async fn valid_session(state: &Shared, token: Option<&str>) -> bool {
    let Some(token) = token else {
        return false;
    };
    let mut sessions = state.sessions.lock().await;
    sessions.retain(|_, expiry| *expiry > Instant::now());
    sessions.contains_key(token)
}

pub(super) fn same_origin(request: &Request) -> bool {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok());
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok());
    matches!((host, origin), (Some(host), Some(origin)) if origin == format!("http://{host}") || origin == format!("https://{host}"))
}

pub(super) async fn login_page() -> impl IntoResponse {
    ([(header::CACHE_CONTROL, "no-store")], Html("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Moderator sign in</title><link rel=\"stylesheet\" href=\"/dashboard/static/css/styles.css\"><main><h1>Moderator sign in</h1><form method=\"post\" action=\"/dashboard/login\"><label for=\"token\">Moderator token</label><input id=\"token\" name=\"token\" type=\"password\" required autocomplete=\"current-password\"><button type=\"submit\">Sign in</button></form></main></html>"))
}

#[derive(serde::Deserialize)]
pub(super) struct Login {
    token: String,
}

pub(super) async fn login(
    State(state): State<Shared>,
    headers: axum::http::HeaderMap,
    Form(input): Form<Login>,
) -> Response {
    let mut request = Request::new(axum::body::Body::empty());
    *request.headers_mut() = headers;
    if !same_origin(&request) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !state
        .config
        .moderator_tokens
        .iter()
        .fold(false, |found, expected| {
            secret_matches(expected, &input.token) | found
        })
    {
        return (StatusCode::UNAUTHORIZED, "Invalid moderator token").into_response();
    }
    let mut sessions = state.sessions.lock().await;
    sessions.retain(|_, expiry| *expiry > Instant::now());
    if sessions.len() >= 1024 {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let token = uuid::Uuid::new_v4().to_string();
    sessions.insert(token.clone(), Instant::now() + Duration::from_secs(28800));
    let secure = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("https://"));
    let cookie = format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=28800{}",
        if secure { "; Secure" } else { "" }
    );
    (
        [
            (header::SET_COOKIE, cookie),
            (header::CACHE_CONTROL, "no-store".into()),
            (header::LOCATION, "/dashboard".into()),
        ],
        StatusCode::SEE_OTHER,
    )
        .into_response()
}

pub(super) async fn logout(State(state): State<Shared>, request: Request) -> Response {
    if let Some(token) = session_token(&request) {
        state.sessions.lock().await.remove(&token);
    }
    (
        [
            (
                header::SET_COOKIE,
                "hre_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
            ),
            (header::LOCATION, "/dashboard/login"),
        ],
        StatusCode::SEE_OTHER,
    )
        .into_response()
}

impl DashboardBackend for AppState {
    fn alerts(&self) -> DashboardFuture<'_, Vec<hre_core::AnalyzeResponse>> {
        Box::pin(async {
            self.engine
                .alerts()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        })
    }
    fn health(&self) -> DashboardFuture<'_, DashboardHealth> {
        Box::pin(async {
            Ok(DashboardHealth {
                engine: hre_core::HealthResponse {
                    status: "ok".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    mock: self.mock,
                    degraded: if self.mock {
                        vec!["mock.no_inference".into()]
                    } else {
                        vec![]
                    },
                },
                models: self.engine.model_health(),
            })
        })
    }
    fn metrics(&self) -> DashboardFuture<'_, Option<DashboardMetrics>> {
        // Historical evaluation is intentionally not presented as live model accuracy.
        Box::pin(async { Ok(None) })
    }
    fn review(&self, id: uuid::Uuid, _request: hre_core::ReviewRequest) -> DashboardFuture<'_, ()> {
        Box::pin(async move {
            let alerts = self
                .engine
                .alerts()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if !alerts.iter().any(|a| a.alert_id == Some(id)) {
                return Err(StatusCode::NOT_FOUND);
            }
            Err(StatusCode::NOT_IMPLEMENTED)
        })
    }
}
