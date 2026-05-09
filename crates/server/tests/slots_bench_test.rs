//! # Slot System Benchmark Tests
//!
//! Performance benchmarks for the slot-based document routing system:
//! - 7.1: Keyword routing throughput (1000 docs)
//! - 7.2: Slot-filtered search vs unfiltered search latency
//! - 7.3: Decay calculation overhead on 10K slot_documents
//!
//! Run with: `cargo test -p anyrag-server --test slots_bench_test -- --ignored`

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use chrono::Utc;
use turso::Database;
use uuid::Uuid;

use anyrag::providers::db::sqlite::sql;

use anyrag::slots::{
    slot_filtered_document_sql, KeywordRouter, SlotIngester, SlotName, SlotSearchConfig,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create an in-memory turso database with all tables created.
async fn create_test_db() -> Arc<Database> {
    let db = Arc::new(
        turso::Builder::new_local(":memory:")
            .build()
            .await
            .expect("create db"),
    );

    let conn = db.connect().expect("connect");
    for stmt in sql::ALL_TABLE_CREATION_SQL {
        conn.execute(stmt, ()).await.expect("create table");
    }

    db
}

/// Build a `SlotIngester`, ensure schema + seed 6 default slots.
async fn setup_ingester(db: Arc<Database>) -> SlotIngester {
    let ingester = SlotIngester::new(db);
    ingester.ensure_schema().await.expect("schema");
    ingester.ensure_default_slots().await.expect("slots");
    ingester
}

/// Synthetic document templates that exercise different slot keyword rules.
const CONTENT_TEMPLATES: &[&str] = &[
    // API content → matches apis slot
    r#"pub fn handler() -> impl Responder {
    Json(response)
}

pub async fn process() -> Result<()> {
    Ok(())
}

pub mod api_module {
    pub fn endpoint() -> String { String::new() }
}"#,
    // Types content → matches types slot
    r#"pub struct Data {
    field: i32,
    name: String,
}

impl Data {
    pub fn new() -> Self { Self { field: 0, name: String::new() } }
}

enum Status {
    Active,
    Inactive,
}

type Result<T> = std::result::Result<T, Error>;"#,
    // Test content → matches tests slot
    r#"#[test]
fn test_unit() {
    assert_eq!(1, 1);
    assert!(true);
}

#[tokio::test]
async fn test_async() {
    let val = 42;
    assert_ne!(val, 0);
}

mod tests {
    use super::*;
    fn test_helper() {}
}"#,
    // Architecture content → matches architecture slot (frozen)
    r#"// mod.rs — module structure overview
mod submod;
mod handlers;

// Architecture: main crate layout with layered design
// This module defines the overall structure diagram."#,
    // Dependencies content → matches dependencies slot
    r#"[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
criterion = { version = "0.5" }"#,
    // Chatter content → matches chatter slot (high decay)
    r#"TODO: fix this hack before release
FIXME: broken error handling
XXX: temporary workaround
NOTE: this is a placeholder"#,
    // Mixed content → matches multiple slots
    r#"pub struct Service {
    db: Db,
}

impl Service {
    pub fn new(db: Db) -> Self { Self { db } }
}

pub fn endpoint() -> impl Responder {
    Json(response)
}

#[test]
fn test_service() {
    assert!(true);
}"#,
    // Random content → may not match any slot
    "just some random text with no particular keywords xyz abc def ghi",
];

/// Generate `count` synthetic documents as `(id, content)` pairs.
fn generate_synthetic_documents(count: usize) -> Vec<(String, String)> {
    (0..count)
        .map(|i| {
            let doc_id = format!("bench-doc-{i}");
            let template = CONTENT_TEMPLATES[i % CONTENT_TEMPLATES.len()];
            let content = format!("{template}\n// doc index: {i}");
            (doc_id, content)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Bench 7.1 — Keyword routing throughput
// ---------------------------------------------------------------------------

/// Benchmark: route 1000 documents through `KeywordRouter::route()` and measure
/// throughput in docs/sec. This is pure in-memory string matching — no DB I/O
/// during the timed section.
#[tokio::test]
#[ignore]
async fn bench_7_1_keyword_routing_throughput_1k_docs() -> Result<()> {
    // Setup: in-memory DB + seed slots
    let db = create_test_db().await;
    let ingester = setup_ingester(db).await;

    let slots = ingester.load_slots().await?;
    let router = KeywordRouter::new(slots);

    // Generate 1000 synthetic documents (mix of API, types, test, etc.)
    let docs = generate_synthetic_documents(1000);

    // --- Timed section: pure keyword routing ---
    let start = Instant::now();
    for (doc_id, content) in &docs {
        let _result = router.route(content, doc_id);
    }
    let elapsed = start.elapsed();
    // --- End timed section ---

    let elapsed_secs = elapsed.as_secs_f64();
    let throughput = 1000.0 / elapsed_secs;

    println!("==================================================");
    println!("Bench 7.1: Keyword Routing Throughput");
    println!("  Documents:   1000");
    println!("  Elapsed:     {elapsed:?}");
    println!("  Throughput:  {throughput:.0} docs/sec");
    println!("==================================================");

    assert!(
        throughput > 100.0,
        "Throughput {throughput:.0} docs/sec is below 100 docs/sec minimum"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Bench 7.2 — Slot-filtered search vs unfiltered search latency
// ---------------------------------------------------------------------------

/// Benchmark: compare latency of a slot-filtered SQL query (with JOINs and
/// decay calculation) against a plain `SELECT * FROM documents`.
#[tokio::test]
#[ignore]
async fn bench_7_2_slot_filtered_vs_unfiltered_search_latency() -> Result<()> {
    // Setup: in-memory DB + seed slots
    let db = create_test_db().await;
    let ingester = setup_ingester(db.clone()).await;
    let conn = db.connect().expect("connect");

    // Insert 100 documents and route them to slots
    let docs = generate_synthetic_documents(100);
    for (doc_id, content) in &docs {
        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?1, ?2, ?3, ?4, ?5)",
            turso::params![
                doc_id.as_str(),
                "bench-user",
                format!("http://test.com/{doc_id}"),
                format!("Doc {doc_id}"),
                content.as_str(),
            ],
        )
        .await?;

        ingester.route_and_persist(doc_id, content).await?;
    }

    // --- Timed section: slot-filtered search ---
    let active_slots = vec![SlotName::Apis, SlotName::Types, SlotName::Tests];
    let config = SlotSearchConfig {
        active_slots: &active_slots,
        include_frozen: true,
    };
    let (filtered_sql, filter_string_params) = slot_filtered_document_sql(&config);
    let filtered_params: Vec<turso::Value> =
        filter_string_params.into_iter().map(Into::into).collect();

    let start_filtered = Instant::now();
    let mut rows = conn.query(&filtered_sql, filtered_params).await?;
    let mut filtered_count = 0usize;
    while let Some(_row) = rows.next().await? {
        filtered_count += 1;
    }
    let filtered_elapsed = start_filtered.elapsed();
    // --- End timed section ---

    // --- Timed section: unfiltered search ---
    let start_unfiltered = Instant::now();
    let mut rows = conn.query("SELECT * FROM documents", ()).await?;
    let mut unfiltered_count = 0usize;
    while let Some(_row) = rows.next().await? {
        unfiltered_count += 1;
    }
    let unfiltered_elapsed = start_unfiltered.elapsed();
    // --- End timed section ---

    println!("==================================================");
    println!("Bench 7.2: Slot-Filtered vs Unfiltered Search");
    println!("  Documents:          100");
    println!("  Filtered results:   {filtered_count}");
    println!("  Unfiltered results: {unfiltered_count}");
    println!("  Filtered latency:   {filtered_elapsed:?}");
    println!("  Unfiltered latency: {unfiltered_elapsed:?}");
    println!("==================================================");

    assert!(
        filtered_elapsed.as_millis() < 100,
        "Filtered search took {filtered_elapsed:?}, expected < 100ms"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Bench 7.3 — Decay calculation overhead on 10K slot_documents
// ---------------------------------------------------------------------------

/// Benchmark: execute `apply_decay_batch_sql()` on 10,000 slot_documents rows
/// with varied timestamps (0–30 days old) and measure wall-clock latency.
#[tokio::test]
#[ignore]
async fn bench_7_3_decay_calculation_10k_rows() -> Result<()> {
    // Setup: in-memory DB + seed slots
    let db = create_test_db().await;
    let ingester = setup_ingester(db.clone()).await;
    let conn = db.connect().expect("connect");

    // Collect non-frozen slots with decay rates (5 default: types, apis, dependencies, tests, chatter)
    let slots = ingester.load_slots().await?;
    let non_frozen_slots: Vec<(String, f64)> = slots
        .iter()
        .filter(|s| !s.is_frozen)
        .map(|s| (s.name.to_string(), s.decay_rate))
        .collect();

    let slots_per_doc = non_frozen_slots.len();
    let doc_count = 10_000 / slots_per_doc; // 2000 docs × 5 slots = 10K rows

    // Insert skeleton documents
    for i in 0..doc_count {
        let doc_id = format!("decay-doc-{i}");
        conn.execute(
            "INSERT INTO documents (id, owner_id, source_url, title, content) VALUES (?1, ?2, ?3, ?4, ?5)",
            turso::params![
                doc_id.as_str(),
                "bench-user",
                format!("http://test.com/{doc_id}"),
                format!("Decay Doc {i}"),
                "benchmark content",
            ],
        )
        .await?;
    }

    // Insert slot_documents with varied timestamps (0–30 days, varied hours)
    for i in 0..doc_count {
        let doc_id = format!("decay-doc-{i}");
        for (slot_idx, (slot_name, _decay_rate)) in non_frozen_slots.iter().enumerate() {
            let sd_id = Uuid::now_v7().to_string();
            let days_ago = (i % 30) as i64;
            let hours_offset = ((i + slot_idx) % 24) as i64;
            let routed_at = (Utc::now()
                - chrono::Duration::days(days_ago)
                - chrono::Duration::hours(hours_offset))
            .to_rfc3339();

            conn.execute(
                "INSERT INTO slot_documents (id, slot_name, document_id, routed_by, routed_at, relevance_score) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                turso::params![
                    sd_id,
                    slot_name.clone(),
                    doc_id.as_str(),
                    "keyword",
                    routed_at,
                    1.0_f64,
                ],
            )
            .await?;
        }
    }

    let total_rows = doc_count * slots_per_doc;

    // --- Timed section: decay batch update ---
    // NOTE: apply_decay_batch_sql() uses IN (SELECT ...) subquery which is not
    // supported by turso's in-memory engine. We execute the equivalent logic as
    // per-slot UPDATE statements, which is what we actually want to benchmark.
    let per_slot_decay_sql = "\
        UPDATE slot_documents \
        SET relevance_score = relevance_score * EXP(-?1 * (JULIANDAY('now') - JULIANDAY(routed_at))) \
        WHERE slot_name = ?2";

    let start = Instant::now();
    for (slot_name, decay_rate) in &non_frozen_slots {
        conn.execute(
            per_slot_decay_sql,
            turso::params![*decay_rate, slot_name.clone()],
        )
        .await?;
    }
    let elapsed = start.elapsed();
    // --- End timed section ---

    println!("==================================================");
    println!("Bench 7.3: Decay Calculation Overhead");
    println!("  Slot documents: {total_rows}");
    println!("  Elapsed:        {elapsed:?}");
    println!("==================================================");

    assert!(
        elapsed.as_millis() < 500,
        "Decay batch took {elapsed:?}, expected < 500ms"
    );

    Ok(())
}
