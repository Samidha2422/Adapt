use crate::{json_error, ApiError, Shared};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};
use hre_core::{AnalyzeResponse, Problem};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CallImportRequest {
    platform: String,
    conversation_id: String,
    /// UTF-8 CSV using hre-calls' nine required columns. Maximum two MiB.
    csv: String,
    #[serde(default)]
    draft: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum RowOutcome {
    Imported,
    Cached,
    Draft,
    Failed,
}

#[derive(Serialize, ToSchema)]
struct CallImportRow {
    /// One-based CSV line, counting the header as line one.
    row: usize,
    event_id: uuid::Uuid,
    outcome: RowOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    assessment: Option<AnalyzeResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<Problem>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct CallImportResponse {
    /// Always false: each row commits independently after the whole CSV validates.
    atomic: bool,
    rows: Vec<CallImportRow>,
}

#[utoipa::path(post, path = "/v1/calllogs/import", request_body = CallImportRequest,
    description = "Validates the entire CSV before analysis. Rows then run in file order through existing behavior/fusion rules with independent atomic event persistence. Import is not all-or-nothing: row failures do not roll back successful rows or stop later rows. Drafts write nothing. Call model and ASR remain unavailable.",
    responses((status = 200, body = CallImportResponse), (status = 400, body = Problem),
        (status = 401, body = Problem), (status = 413, body = Problem),
        (status = 422, body = Problem), (status = 501, body = Problem)),
    security(("api_key" = []), ("moderator" = [])))]
pub(crate) async fn import_calls(
    State(state): State<Shared>,
    input: Result<Json<CallImportRequest>, JsonRejection>,
) -> Result<Json<CallImportResponse>, ApiError> {
    let Json(request) = input.map_err(json_error)?;
    if state.mock {
        return Err(ApiError::new(
            StatusCode::NOT_IMPLEMENTED,
            "CSV import requires the real engine",
        ));
    }
    if request.conversation_id.trim().is_empty()
        || request.conversation_id.len() > 256
        || request.conversation_id.chars().any(char::is_control)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid conversation_id",
        ));
    }
    if request.csv.len() > hre_calls::MAX_CSV_BYTES {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "CSV exceeds two MiB",
        ));
    }
    let records = state
        .engine
        .parse_calls(request.csv, request.platform.clone())
        .await?;
    let mut rows = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let result = state
            .engine
            .analyze_call(
                record,
                &request.platform,
                &request.conversation_id,
                request.draft,
            )
            .await;
        let (outcome, assessment, problem) = match result {
            Ok((response, fresh)) => {
                if fresh && response.alert_id.is_some() {
                    let _ = state.alerts.send(response.clone());
                }
                let outcome = if request.draft {
                    RowOutcome::Draft
                } else if fresh {
                    RowOutcome::Imported
                } else {
                    RowOutcome::Cached
                };
                (outcome, Some(response), None)
            }
            Err(error) => (RowOutcome::Failed, None, Some(error.0)),
        };
        rows.push(CallImportRow {
            row: index + 2,
            event_id: record.event_id,
            outcome,
            assessment,
            problem,
        });
    }
    Ok(Json(CallImportResponse {
        atomic: false,
        rows,
    }))
}
