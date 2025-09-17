use crate::auth::middleware::AuthenticatedUser;
use crate::handlers::ingest::firebase_types::{IngestFirebaseRequest, IngestFirebaseResponse};
use crate::handlers::{
    graph_handlers, wrap_response, ApiResponse, AppError, AppState, DebugParams,
};
use anyhow::anyhow;
use anyrag::ingest::Ingestor;
use anyrag::providers::factory::create_dynamic_provider;
use anyrag_firebase::{sanitize_table_name, FirebaseIngestor, FirebaseSource};
use anyrag_web::extract_and_store_metadata;
use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::json;
use tracing::{info, warn};
use turso::Value as TursoValue;
use uuid::Uuid;

impl From<&IngestFirebaseRequest> for FirebaseSource {
    fn from(req: &IngestFirebaseRequest) -> Self {
        Self {
            project_id: req.project_id.clone(),
            collection: req.collection.clone(),
            incremental: req.incremental,
            timestamp_field: req.timestamp_field.clone(),
            limit: req.limit,
            fields: req.fields.clone(),
        }
    }
}

/// Handler for ingesting a Firestore collection into a local project database.
pub async fn ingest_firebase_handler(
    State(app_state): State<AppState>,
    user: AuthenticatedUser,
    debug_params: Query<DebugParams>,
    Json(payload): Json<IngestFirebaseRequest>,
) -> Result<Json<ApiResponse<IngestFirebaseResponse>>, AppError> {
    let owner_id = Some(user.0.id.clone());
    info!(
        "Received Firestore ingest request for project: '{}', collection: '{}'",
        payload.project_id, payload.collection
    );

    let db_path = format!("{}/{}.db", anyrag::constants::DB_DIR, payload.project_id);
    let sqlite_provider = anyrag::providers::db::sqlite::SqliteProvider::new(&db_path).await?;
    sqlite_provider.initialize_schema().await?;

    let firebase_source = FirebaseSource::from(&*payload);
    let source_str = serde_json::to_string(&firebase_source).map_err(|e| {
        AppError::Internal(anyhow!(
            "Failed to serialize Firebase source for project '{}': {}",
            payload.project_id,
            e
        ))
    })?;

    let ingestor = FirebaseIngestor::new(&sqlite_provider);
    let ingestion_result = ingestor
        .ingest(&source_str, owner_id.as_deref())
        .await
        .map_err(|e| {
            AppError::Internal(anyhow!(
                "Firebase ingestion failed for project '{}' and collection '{}': {}",
                payload.project_id,
                payload.collection,
                e
            ))
        })?;

    let ingested_count = ingestion_result.documents_added;

    if ingested_count == 0 {
        let response = IngestFirebaseResponse {
            message: "No new documents to ingest from Firestore.".to_string(),
            ingested_documents: 0,
            documents_processed_for_metadata: 0,
            facts_added_to_graph: None,
        };
        return Ok(wrap_response(response, debug_params, None));
    }

    let table_name = sanitize_table_name(&payload.collection);
    let conn = sqlite_provider.db.connect()?;

    let source_url_prefix = format!("db://{}/{}%", payload.project_id, &table_name);
    conn.execute(
        "DELETE FROM documents WHERE source_url LIKE ?",
        turso::params![source_url_prefix],
    )
    .await?;
    info!(
        "Cleared old shadow documents for collection '{}' before ingestion.",
        payload.collection
    );

    let meta_task_config = app_state
        .tasks
        .get("knowledge_metadata_extraction")
        .unwrap();
    let (meta_ai_provider, _) = if let Some(model_name) = &payload.model {
        create_dynamic_provider(&app_state.config.providers, model_name)?
    } else {
        let provider_name = &meta_task_config.provider;
        let provider = app_state
            .ai_providers
            .get(provider_name)
            .ok_or_else(|| {
                AppError::Internal(anyhow!(
                    "Provider '{provider_name}' for task 'knowledge_metadata_extraction' not found in providers map."
                ))
            })?
            .clone();
        let provider_config = app_state.config.providers.get(provider_name).unwrap();
        (provider, provider_config.model_name.clone())
    };

    let all_data_sql = format!("SELECT * FROM \"{table_name}\"");
    let mut stmt = conn.prepare(&all_data_sql).await?;
    let column_names: Vec<String> = stmt
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    let mut data_rows = stmt.query(()).await?;
    let mut documents_processed_for_metadata = 0;
    let id_col_index = column_names.iter().position(|name| name == "_id");

    let turso_value_to_string = |val: TursoValue| -> String {
        match val {
            TursoValue::Text(s) => s,
            TursoValue::Integer(i) => i.to_string(),
            TursoValue::Real(f) => f.to_string(),
            _ => "".to_string(),
        }
    };

    while let Some(row) = data_rows.next().await? {
        let mut document_content_parts = Vec::new();
        let mut title = String::new();

        let pk_val = id_col_index
            .and_then(|index| row.get_value(index).ok())
            .map(turso_value_to_string);

        let pk_val = match pk_val {
            Some(pk) if !pk.is_empty() => pk,
            _ => {
                warn!(
                    "Skipping row in table '{table_name}' due to missing or invalid primary key (_id)."
                );
                continue;
            }
        };

        for (i, name) in column_names.iter().enumerate() {
            let value_str = turso_value_to_string(row.get_value(i)?);
            if !value_str.is_empty() {
                if name.to_lowercase() == "title" {
                    title = value_str.clone();
                }
                document_content_parts.push(format!("{name}: {value_str}"));
            }
        }

        if title.is_empty() {
            title = pk_val.clone();
        }
        let document_content = document_content_parts.join("\n\n");
        let source_url = format!("db://{}/{}/{}", payload.project_id, table_name, pk_val);
        let document_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, source_url.as_bytes()).to_string();

        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?, ?, ?, ?, ?)",
            turso::params![document_id.clone(), owner_id.clone(), source_url, title, document_content.clone()],
        )
        .await?;

        if let Err(e) = extract_and_store_metadata(
            &conn,
            meta_ai_provider.as_ref(),
            &document_id,
            owner_id.as_deref(),
            &document_content,
            &meta_task_config.system_prompt,
        )
        .await
        {
            info!("Could not extract metadata for doc {document_id}: {e}");
        }
        documents_processed_for_metadata += 1;
    }
    info!("Processed {documents_processed_for_metadata} documents for metadata extraction.");

    let mut facts_added_to_graph = None;
    if payload.use_graph {
        info!("`use_graph` is true. Triggering knowledge graph build for table '{table_name}'.");
        let graph_build_payload = graph_handlers::GraphBuildRequest {
            db: payload.project_id.clone(),
            table_name: table_name.clone(),
        };
        let graph_debug_params = Query(DebugParams {
            debug: debug_params.0.debug,
        });
        let graph_response = graph_handlers::graph_build_handler(
            State(app_state),
            user,
            graph_debug_params,
            Json(graph_build_payload),
        )
        .await?;
        facts_added_to_graph = Some(graph_response.0.result.facts_added);
    }

    let response = IngestFirebaseResponse {
        message: format!(
            "Successfully ingested and processed {ingested_count} documents from Firestore."
        ),
        ingested_documents: ingested_count,
        documents_processed_for_metadata,
        facts_added_to_graph,
    };

    let debug_info = json!({
        "project_id": payload.project_id,
        "collection": payload.collection,
        "incremental": payload.incremental,
        "timestamp_field": payload.timestamp_field,
        "limit": payload.limit,
        "fields": payload.fields,
        "use_graph": payload.use_graph,
        "generated_table_name": table_name,
    });

    Ok(wrap_response(response, debug_params, Some(debug_info)))
}
