//! In-process bridge implemented by the API owner. Authentication stays in the host router.

use std::{future::Future, pin::Pin, sync::Arc};

use axum::http::StatusCode;
use hre_core::{AnalyzeResponse, HealthResponse, ModelHealth, ReviewRequest};
use uuid::Uuid;

/// Object-safe async result; errors must not contain private payloads.
pub type DashboardFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, StatusCode>> + Send + 'a>>;

#[derive(Clone)]
pub struct DashboardState(pub Arc<dyn DashboardBackend>);

pub struct DashboardHealth {
    pub engine: HealthResponse,
    pub models: Vec<ModelHealth>,
}

/// A measured evaluation summary. Missing measurements stay absent, never zero-filled.
#[derive(Default)]
pub struct DashboardMetrics {
    pub source: String,
    pub simulated: bool,
    pub samples: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub detection_rate: Option<f64>,
    pub time_to_detection_events: Option<f64>,
    /// TP, FP, FN, TN, in that order.
    pub confusion: Option<[u32; 4]>,
    pub latency_p50_ms: Option<f64>,
    pub latency_p95_ms: Option<f64>,
    pub latency_p99_ms: Option<f64>,
}

/// Supply persisted assessments and runtime health, never fabricated UI fixtures.
pub trait DashboardBackend: Send + Sync {
    fn alerts(&self) -> DashboardFuture<'_, Vec<AnalyzeResponse>>;
    fn health(&self) -> DashboardFuture<'_, DashboardHealth>;
    fn metrics(&self) -> DashboardFuture<'_, Option<DashboardMetrics>>;
    /// Return success only after the review has actually persisted.
    fn review(&self, id: Uuid, request: ReviewRequest) -> DashboardFuture<'_, ()>;
}
