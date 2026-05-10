//! # Slot Management & Search Handlers
//!
//! API endpoints for managing Raven Routed Slots and slot-based search.
//! Provides CRUD for slots, document-to-slot management, reindexing,
//! and slot-filtered search with selective decay scoring.

use super::{wrap_response, ApiResponse, AppError, AppState, DebugParams};
use crate::auth::middleware::AuthenticatedUser;
use anyrag::slots::{
    seed_default_slots, slot_filtered_document_sql, KeywordRouter, Slot, SlotName, SlotSearchConfig,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

// === Request Types ===

#[derive(Debug, Deserialize)]
pub struct CreateSlotRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f64,
    pub keywords: Vec<String>,
}

fn default_decay_rate() -> f64 {
    0.1
}

#[derive(Debug, Deserialize)]
pub struct SlotSearchRequest {
    pub active_slots: Vec<SlotName>,
    #[serde(default = "default_true")]
    pub include_frozen: bool,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_true() -> bool {
    true
}

fn default_limit() -> u32 {
    10
}

// === Response Types ===

#[derive(Debug, Serialize)]
pub struct SlotInfo {
    pub name: String,
    pub description: String,
    pub is_frozen: bool,
    pub decay_rate: f64,
    pub max_documents: i64,
    pub document_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SlotDocumentInfo {
    pub id: String,
    pub title: String,
    pub decayed_score: f64,
    pub routed_at: String,
    pub routed_by: String,
}

#[derive(Debug, Serialize)]
pub struct SlotSearchResponse {
    pub results: Vec<SlotSearchResultItem>,
    pub active_slots: Vec<String>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SlotSearchResultItem {
    pub id: String,
    pub title: String,
    pub source_url: String,
    pub slot_score: f64,
}

#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    pub total_documents_routed: usize,
    pub total_routing_entries: usize,
    pub total_documents_scanned: usize,
}

#[derive(Debug, Serialize)]
pub struct CreateSlotResponse {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RemoveDocumentResponse {
    pub removed: bool,
}

// === Handlers ===

/// GET /slots — List all slots with document counts.
pub async fn list_slots_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<Vec<SlotInfo>>>, AppError> {
    info!("Listing all slots");
    let conn = app_state.sqlite_provider.db.connect()?;

    let mut rows = conn
        .query(
            "SELECT s.name, s.description, s.is_frozen, s.decay_rate, s.max_documents, \
             COALESCE(COUNT(sd.id), 0) as doc_count \
             FROM rag_slots s \
             LEFT JOIN slot_documents sd ON sd.slot_name = s.name \
             GROUP BY s.name, s.description, s.is_frozen, s.decay_rate, s.max_documents \
             ORDER BY s.name",
            (),
        )
        .await?;

    let mut slots = Vec::new();
    while let Some(row) = rows.next().await? {
        slots.push(SlotInfo {
            name: row.get(0).unwrap_or_default(),
            description: row.get(1).unwrap_or_default(),
            is_frozen: row.get::<i64>(2).unwrap_or(0) != 0,
            decay_rate: row.get(3).unwrap_or(0.1),
            max_documents: row.get(4).unwrap_or(1000),
            document_count: row.get(5).unwrap_or(0),
        });
    }

    // Seed defaults if no slots exist
    if slots.is_empty() {
        drop(rows);
        let defaults = seed_default_slots();
        for slot in &defaults {
            let keywords_json = serde_json::to_string(&slot.keywords)?;
            let params: Vec<turso::Value> = vec![
                slot.name.to_string().into(),
                slot.description.clone().into(),
                (if slot.is_frozen { 1 } else { 0 }).into(),
                slot.decay_rate.into(),
                (slot.max_documents as i64).into(),
                keywords_json.into(),
                slot.created_at.clone().into(),
                slot.updated_at.clone().into(),
            ];
            conn.execute(
                "INSERT INTO rag_slots (name, description, is_frozen, decay_rate, max_documents, keywords, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params,
            )
            .await?;
        }

        // Re-query after seeding
        let mut rows = conn
            .query(
                "SELECT s.name, s.description, s.is_frozen, s.decay_rate, s.max_documents, \
                 COALESCE(COUNT(sd.id), 0) as doc_count \
                 FROM rag_slots s \
                 LEFT JOIN slot_documents sd ON sd.slot_name = s.name \
                 GROUP BY s.name, s.description, s.is_frozen, s.decay_rate, s.max_documents \
                 ORDER BY s.name",
                (),
            )
            .await?;

        while let Some(row) = rows.next().await? {
            slots.push(SlotInfo {
                name: row.get(0).unwrap_or_default(),
                description: row.get(1).unwrap_or_default(),
                is_frozen: row.get::<i64>(2).unwrap_or(0) != 0,
                decay_rate: row.get(3).unwrap_or(0.1),
                max_documents: row.get(4).unwrap_or(1000),
                document_count: row.get(5).unwrap_or(0),
            });
        }
    }

    Ok(wrap_response(slots, debug_params, None))
}

/// POST /slots — Create a custom slot.
pub async fn create_slot_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateSlotRequest>,
) -> Result<Json<ApiResponse<CreateSlotResponse>>, AppError> {
    let name = payload.name.to_lowercase().replace(' ', "_");
    info!("Creating custom slot: {name}");

    let conn = app_state.sqlite_provider.db.connect()?;

    // Check if slot already exists
    let params: Vec<turso::Value> = vec![name.clone().into()];
    let mut rows = conn
        .query(
            "SELECT COUNT(*) as cnt FROM rag_slots WHERE name = ?1",
            params,
        )
        .await?;

    let count = if let Some(row) = rows.next().await? {
        row.get::<i64>(0).unwrap_or(0)
    } else {
        0
    };

    if count > 0 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Slot '{name}' already exists"
        )));
    }

    let keywords_json = serde_json::to_string(&payload.keywords)?;
    let now = chrono::Utc::now().to_rfc3339();

    let params: Vec<turso::Value> = vec![
        name.clone().into(),
        payload.description.into(),
        0i64.into(), // is_frozen = false for custom slots
        payload.decay_rate.into(),
        1000i64.into(), // max_documents
        keywords_json.into(),
        now.clone().into(),
        now.into(),
    ];

    conn.execute(
        "INSERT INTO rag_slots (name, description, is_frozen, decay_rate, max_documents, keywords, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params,
    )
    .await?;

    Ok(Json(ApiResponse {
        debug: None,
        result: CreateSlotResponse {
            name,
            message: "Slot created successfully".to_string(),
        },
    }))
}

/// GET /slots/{name}/documents — List documents in a slot with decayed scores.
pub async fn list_slot_documents_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    Path(slot_name): Path<String>,
    debug_params: Query<DebugParams>,
) -> Result<Json<ApiResponse<Vec<SlotDocumentInfo>>>, AppError> {
    info!("Listing documents in slot: {slot_name}");

    let conn = app_state.sqlite_provider.db.connect()?;

    let params: Vec<turso::Value> = vec![slot_name.into()];
    let mut rows = conn
        .query(
            "SELECT d.id, COALESCE(d.title, ''), \
             CASE \
                WHEN s.is_frozen = TRUE THEN sd.relevance_score \
                ELSE sd.relevance_score * EXP(-s.decay_rate * (JULIANDAY('now') - JULIANDAY(sd.routed_at))) \
             END AS decayed_score, \
             sd.routed_at, sd.routed_by \
             FROM documents d \
             JOIN slot_documents sd ON sd.document_id = d.id \
             JOIN rag_slots s ON s.name = sd.slot_name \
             WHERE sd.slot_name = ?1 \
             ORDER BY decayed_score DESC",
            params,
        )
        .await?;

    let mut documents = Vec::new();
    while let Some(row) = rows.next().await? {
        documents.push(SlotDocumentInfo {
            id: row.get(0).unwrap_or_default(),
            title: row.get(1).unwrap_or_default(),
            decayed_score: row.get(2).unwrap_or(0.0),
            routed_at: row.get(3).unwrap_or_default(),
            routed_by: row.get(4).unwrap_or_default(),
        });
    }

    Ok(wrap_response(documents, debug_params, None))
}

/// DELETE /slots/{name}/documents/{doc_id} — Remove document from slot.
pub async fn remove_document_from_slot_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    Path((slot_name, doc_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<RemoveDocumentResponse>>, AppError> {
    info!("Removing document {doc_id} from slot {slot_name}");

    let conn = app_state.sqlite_provider.db.connect()?;

    let params: Vec<turso::Value> = vec![slot_name.into(), doc_id.into()];
    conn.execute(
        "DELETE FROM slot_documents WHERE slot_name = ?1 AND document_id = ?2",
        params,
    )
    .await?;

    Ok(Json(ApiResponse {
        debug: None,
        result: RemoveDocumentResponse { removed: true },
    }))
}

/// POST /slots/reindex — Re-route all documents through keyword router.
pub async fn reindex_slots_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ApiResponse<ReindexResponse>>, AppError> {
    info!("Reindexing all documents through slot router");

    let conn = app_state.sqlite_provider.db.connect()?;

    // 1. Load slot definitions
    let mut slot_rows = conn
        .query(
            "SELECT name, keywords, description, is_frozen, decay_rate FROM rag_slots",
            (),
        )
        .await?;

    let mut slot_defs = Vec::new();
    while let Some(row) = slot_rows.next().await? {
        let name_str: String = row.get(0).unwrap_or_default();
        let keywords_json: String = row.get(1).unwrap_or("[]".to_string());
        let description: String = row.get(2).unwrap_or_default();
        let is_frozen: i64 = row.get(3).unwrap_or(0);
        let decay_rate: f64 = row.get(4).unwrap_or(0.1);
        let keywords: Vec<String> = serde_json::from_str(&keywords_json).unwrap_or_default();

        slot_defs.push(Slot {
            id: String::new(),
            name: parse_slot_name(&name_str),
            description,
            is_frozen: is_frozen != 0,
            decay_rate,
            max_documents: 1000,
            keywords,
            created_at: String::new(),
            updated_at: String::new(),
        });
    }

    if slot_defs.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "No slots defined. Create slots before reindexing."
        )));
    }

    let router = KeywordRouter::new(slot_defs);

    // 2. Load all documents
    let mut doc_rows = conn.query("SELECT id, content FROM documents", ()).await?;
    let mut documents = Vec::new();
    while let Some(row) = doc_rows.next().await? {
        let id: String = row.get(0).unwrap_or_default();
        let content: String = row.get(1).unwrap_or_default();
        documents.push((id, content));
    }

    // 3. Clear existing slot_documents
    conn.execute("DELETE FROM slot_documents", ()).await?;

    // 4. Re-route each document
    let mut total_documents_routed = 0usize;
    let mut total_routing_entries = 0usize;
    let now = chrono::Utc::now().to_rfc3339();

    for (doc_id, content) in &documents {
        let route_result = router.route(content, doc_id);
        if route_result.assigned_slots.is_empty() {
            continue;
        }

        total_documents_routed += 1;

        for slot_name in &route_result.assigned_slots {
            let id = uuid::Uuid::now_v7().to_string();
            let params: Vec<turso::Value> = vec![
                id.into(),
                slot_name.to_string().into(),
                doc_id.clone().into(),
                "keyword".into(),
                now.clone().into(),
                1.0f64.into(),
            ];

            conn.execute(
                "INSERT INTO slot_documents (id, slot_name, document_id, routed_by, routed_at, relevance_score) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params,
            )
            .await?;

            total_routing_entries += 1;
        }
    }

    let result = ReindexResponse {
        total_documents_routed,
        total_routing_entries,
        total_documents_scanned: documents.len(),
    };

    Ok(Json(ApiResponse {
        debug: None,
        result,
    }))
}

/// POST /search/slots — Slot-filtered search with decay scoring.
pub async fn slot_search_handler(
    State(app_state): State<AppState>,
    _user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<SlotSearchRequest>,
) -> Result<Json<ApiResponse<SlotSearchResponse>>, AppError> {
    info!(
        "Slot search with {} active slots",
        payload.active_slots.len()
    );

    let config = SlotSearchConfig {
        active_slots: &payload.active_slots,
        include_frozen: payload.include_frozen,
    };

    let (sql, params) = slot_filtered_document_sql(&config);
    let limit = payload.limit;

    // Add limit to query
    let sql = format!("{sql} LIMIT {limit}");

    let conn = app_state.sqlite_provider.db.connect()?;

    let turso_params: Vec<turso::Value> = params.into_iter().map(turso::Value::from).collect();
    let mut rows = conn.query(&sql, turso_params).await?;

    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        results.push(SlotSearchResultItem {
            id: row.get(0).unwrap_or_default(),
            title: row.get(1).unwrap_or_default(),
            source_url: row.get(3).unwrap_or_default(),
            slot_score: row.get(5).unwrap_or(0.0),
        });
    }

    let active_slot_names: Vec<String> =
        payload.active_slots.iter().map(|s| s.to_string()).collect();
    let total = results.len();

    Ok(wrap_response(
        SlotSearchResponse {
            results,
            active_slots: active_slot_names,
            total,
        },
        debug_params,
        None,
    ))
}

/// Parse a slot name string back into a `SlotName` enum.
fn parse_slot_name(name: &str) -> SlotName {
    match name {
        "architecture" => SlotName::Architecture,
        "types" => SlotName::Types,
        "apis" => SlotName::Apis,
        "dependencies" => SlotName::Dependencies,
        "tests" => SlotName::Tests,
        "chatter" => SlotName::Chatter,
        other => SlotName::Custom(other.to_string()),
    }
}
