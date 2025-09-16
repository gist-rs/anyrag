use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use anyrag::ingest::{
    ingest_faq_from_google_sheet, ingest_from_google_sheet_url,
    sheet_url_to_export_url_and_table_name,
};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize, Serialize, Debug)]
pub struct IngestParams {
    #[serde(default)]
    pub faq: bool,
    #[serde(default = "default_true")]
    pub embed: bool,
}

#[derive(Deserialize)]
pub struct IngestSheetRequest {
    pub url: String,
    #[serde(default)]
    pub gid: Option<String>,
    #[serde(default = "default_true")]
    pub skip_header: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct IngestSheetResponse {
    pub message: String,
    pub ingested_rows: usize,
    pub table_name: Option<String>,
}

/// Unified handler for ingesting a Google Sheet as a generic table or as structured FAQs.
pub async fn ingest_sheet_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    Query(params): Query<IngestParams>,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestSheetRequest>,
) -> Result<Json<ApiResponse<IngestSheetResponse>>, AppError> {
    let owner_id = Some(user.0.id);
    info!(
        "User '{:?}' initiating Sheet ingest for URL: {} with params: faq={}, embed={}",
        owner_id, payload.url, params.faq, params.embed
    );

    let (ingested_count, table_name, message) = if params.faq {
        // --- FAQ Ingestion Path ---
        let count = ingest_faq_from_google_sheet(
            &app_state.sqlite_provider.db,
            &payload.url,
            owner_id.as_deref(),
            payload.gid.as_deref(),
            payload.skip_header,
        )
        .await?;
        (count, None, "Sheet FAQ ingestion successful".to_string())
    } else {
        // --- Generic Table Ingestion Path ---
        let (export_url, table_name) =
            sheet_url_to_export_url_and_table_name(&payload.url).map_err(anyhow::Error::from)?;
        let count =
            ingest_from_google_sheet_url(&app_state.sqlite_provider.db, &export_url, &table_name)
                .await
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Generic sheet ingestion failed: {e}"))
                })?;
        (
            count,
            Some(table_name.clone()),
            format!("Generic sheet ingested successfully into table '{table_name}'."),
        )
    };

    let response = IngestSheetResponse {
        message,
        ingested_rows: ingested_count,
        table_name,
    };

    let debug_info = json!({
        "url": payload.url,
        "gid": payload.gid,
        "skip_header": payload.skip_header,
        "owner_id": owner_id,
        "params": params,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
