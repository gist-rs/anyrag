#[cfg(feature = "graph_db")]
use anyrag::graph::types::{MemoryKnowledgeGraph, RocksdbKnowledgeGraph};
use chrono::{DateTime, Duration, Utc};
use tempfile::{tempdir, TempDir};

/// A local trait to abstract over different KnowledgeGraph implementations.
///
/// This allows writing a single set of test logic that can be applied
/// to any struct that implements this trait. The methods here panic on error
/// to simplify the test logic, assuming tests should not produce errors.
#[cfg(feature = "graph_db")]
trait KnowledgeGraphTest {
    fn add_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    );

    fn get_fact_as_of(
        &self,
        subject: &str,
        predicate: &str,
        as_of: DateTime<Utc>,
    ) -> Option<String>;
}

/// Implements the test trait for the in-memory knowledge graph.
#[cfg(feature = "graph_db")]
impl KnowledgeGraphTest for MemoryKnowledgeGraph {
    fn add_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) {
        self.add_fact(subject, predicate, object, start_time, end_time)
            .expect("Failed to add fact in MemoryKnowledgeGraph");
    }

    fn get_fact_as_of(
        &self,
        subject: &str,
        predicate: &str,
        as_of: DateTime<Utc>,
    ) -> Option<String> {
        self.get_fact_as_of(subject, predicate, as_of)
            .expect("Failed to get fact in MemoryKnowledgeGraph")
    }
}

/// A test harness for `RocksdbKnowledgeGraph`.
///
/// This struct holds both the graph instance and the `TempDir` guard.
/// When an instance of this struct is dropped, the `_temp_dir` field is also
/// dropped, which cleans up the temporary directory from the filesystem.
#[cfg(feature = "graph_db")]
struct RocksDbTestHarness {
    kg: RocksdbKnowledgeGraph,
    _temp_dir: TempDir,
}

#[cfg(feature = "graph_db")]
impl KnowledgeGraphTest for RocksDbTestHarness {
    fn add_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) {
        self.kg
            .add_fact(subject, predicate, object, start_time, end_time)
            .expect("Failed to add fact in RocksdbKnowledgeGraph");
    }

    fn get_fact_as_of(
        &self,
        subject: &str,
        predicate: &str,
        as_of: DateTime<Utc>,
    ) -> Option<String> {
        self.kg
            .get_fact_as_of(subject, predicate, as_of)
            .expect("Failed to get fact in RocksdbKnowledgeGraph")
    }
}

/// Generic test logic for basic fact addition and retrieval.
#[cfg(feature = "graph_db")]
fn run_test_fact_addition_and_retrieval(kg: &mut dyn KnowledgeGraphTest) {
    let now = Utc::now();
    kg.add_fact(
        "subject1",
        "predicate1",
        "object1",
        now,
        now + Duration::days(1),
    );
    let fact = kg.get_fact_as_of("subject1", "predicate1", now);
    assert_eq!(fact, Some("object1".to_string()));
}

/// Generic test logic for time-constrained fact retrieval.
#[cfg(feature = "graph_db")]
fn run_test_time_constrained_fact_retrieval(kg: &mut dyn KnowledgeGraphTest) {
    let now = Utc::now();
    let past_start = now - Duration::days(10);
    let past_end = now - Duration::days(5);
    let current_start = now - Duration::days(1);
    let current_end = now + Duration::days(1);
    let future_start = now + Duration::days(5);
    let future_end = now + Duration::days(10);

    kg.add_fact("Alice", "role", "Developer", past_start, past_end);
    kg.add_fact(
        "Alice",
        "role",
        "Lead Developer",
        current_start,
        current_end,
    );
    kg.add_fact("Alice", "role", "Architect", future_start, future_end);

    let current_role = kg.get_fact_as_of("Alice", "role", now);
    assert_eq!(
        current_role,
        Some("Lead Developer".to_string()),
        "Should retrieve the current role."
    );

    let past_role = kg.get_fact_as_of("Alice", "role", past_start + Duration::days(1));
    assert_eq!(
        past_role,
        Some("Developer".to_string()),
        "Should retrieve the past role."
    );

    let future_role = kg.get_fact_as_of("Alice", "role", future_start + Duration::days(1));
    assert_eq!(
        future_role,
        Some("Architect".to_string()),
        "Should retrieve the future role."
    );

    let no_role_time = now - Duration::days(3);
    let no_role = kg.get_fact_as_of("Alice", "role", no_role_time);
    assert_eq!(no_role, None, "Should retrieve no role.");
}

#[test]
#[cfg(feature = "graph_db")]
fn test_memory_knowledge_graph_suite() {
    run_test_fact_addition_and_retrieval(&mut MemoryKnowledgeGraph::new_memory());
    run_test_time_constrained_fact_retrieval(&mut MemoryKnowledgeGraph::new_memory());
}

#[test]
#[cfg(feature = "graph_db")]
fn test_rocksdb_knowledge_graph_suite() {
    let dir1 = tempdir().unwrap();
    let mut harness1 = RocksDbTestHarness {
        kg: RocksdbKnowledgeGraph::new_rocksdb(dir1.path()).unwrap(),
        _temp_dir: dir1,
    };
    run_test_fact_addition_and_retrieval(&mut harness1);

    let dir2 = tempdir().unwrap();
    let mut harness2 = RocksDbTestHarness {
        kg: RocksdbKnowledgeGraph::new_rocksdb(dir2.path()).unwrap(),
        _temp_dir: dir2,
    };
    run_test_time_constrained_fact_retrieval(&mut harness2);
}
