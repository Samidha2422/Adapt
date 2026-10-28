//! Vendored static assets embedded directly into binary.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

pub const STYLES_CSS: &str = include_str!("../static/css/styles.css");
pub const HTMX_JS: &str = include_str!("../static/vendor/htmx/htmx.min.js");
pub const SSE_JS: &str = include_str!("../static/vendor/htmx/sse.js");
pub const M3E_JS: &[u8] = include_bytes!("../static/vendor/m3e/m3e.min.js");

pub async fn serve_styles() -> Response {
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/css; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        STYLES_CSS,
    )
        .into_response()
}

pub async fn serve_htmx() -> Response {
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/javascript; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            ),
        ],
        HTMX_JS,
    )
        .into_response()
}

pub async fn serve_sse() -> Response {
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/javascript; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            ),
        ],
        SSE_JS,
    )
        .into_response()
}

pub async fn serve_m3e() -> Response {
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/javascript; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            ),
        ],
        M3E_JS,
    )
        .into_response()
}
