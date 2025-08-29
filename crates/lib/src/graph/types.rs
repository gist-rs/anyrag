use chrono::{DateTime, Utc};
use indradb::{Datastore, MemoryDatastore, RocksdbDatastore, ValidationError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum KnowledgeGraphError {
    #[error("IndraDB error: {0}")]
    IndraDb(#[from] indradb::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Identifier validation error: {0}")]
    IdentifierValidation(#[from] ValidationError),
    #[error("Entity '{0}' not found in graph")]
    EntityNotFound(String),
    #[error("Required data was not found in the graph response")]
    NotFound,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TimeConstraint {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

/// A knowledge graph that stores facts with time-based validity, generic
/// over the underlying datastore.
pub struct KnowledgeGraph<D: Datastore> {
    pub db: indradb::Database<D>,
    pub entity_map: HashMap<String, Uuid>,
}

/// Type alias for an in-memory knowledge graph.
pub type MemoryKnowledgeGraph = KnowledgeGraph<MemoryDatastore>;
/// Type alias for a RocksDB-backed knowledge graph.
pub type RocksdbKnowledgeGraph = KnowledgeGraph<RocksdbDatastore>;
