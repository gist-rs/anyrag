//! # State Manager for Incremental Sync
//!
//! This module handles reading and writing the "high-water mark" for incremental
//! data dumps. It saves the most recent timestamp seen for a given collection,
//! allowing subsequent dumps to fetch only newer records.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};
use tracing::info;

/// The structure of the state file.
#[derive(Serialize, Deserialize, Default, Debug)]
struct SyncState(HashMap<String, String>);

/// Reads the last sync timestamp for a specific collection from the state file.
///
/// If the state file or an entry for the collection does not exist, it returns `Ok(None)`.
pub fn read_last_timestamp(project_id: &str, collection_name: &str) -> Result<Option<String>> {
    let state_file_name = format!(".anyrag_sync_state_{project_id}.json");
    let path = Path::new(&state_file_name);
    if !path.exists() {
        info!("Sync state file not found. A full sync will be performed.");
        return Ok(None);
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let state: SyncState = serde_json::from_reader(reader)?;

    let timestamp = state.0.get(collection_name).cloned();
    if let Some(ts) = &timestamp {
        info!(
            "Found last sync timestamp for collection '{}': {}",
            collection_name, ts
        );
    } else {
        info!(
            "No sync timestamp found for collection '{}'. A full sync will be performed.",
            collection_name
        );
    }

    Ok(timestamp)
}

/// Writes the latest sync timestamp for a specific collection to the state file.
///
/// This function reads the existing state, updates the entry for the given
/// collection, and writes the entire state back to the file.
pub fn write_last_timestamp(
    project_id: &str,
    collection_name: &str,
    timestamp: &str,
) -> Result<()> {
    let state_file_name = format!(".anyrag_sync_state_{project_id}.json");
    let path = Path::new(&state_file_name);
    let mut state: SyncState = if path.exists() {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap_or_default()
    } else {
        SyncState::default()
    };

    state
        .0
        .insert(collection_name.to_string(), timestamp.to_string());

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &state)?;

    info!(
        "Updated sync state for collection '{}' to: {}",
        collection_name, timestamp
    );

    Ok(())
}
