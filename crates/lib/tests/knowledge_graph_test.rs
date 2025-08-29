use anyrag::graph::types::MemoryKnowledgeGraph;
use chrono::{Duration, Utc};

#[test]
#[cfg(feature = "graph_db")]
fn test_fact_addition_and_retrieval() {
    let mut kg = MemoryKnowledgeGraph::new_memory();
    let now = Utc::now();
    kg.add_fact(
        "subject1",
        "predicate1",
        "object1",
        now,
        now + Duration::days(1),
    )
    .unwrap();
    let fact = kg.get_fact_as_of("subject1", "predicate1", now).unwrap();
    assert_eq!(fact, Some("object1".to_string()));
}

#[test]
#[cfg(feature = "graph_db")]
fn test_time_constrained_fact_retrieval() {
    let mut kg = MemoryKnowledgeGraph::new_memory();

    let now = Utc::now();
    let past_start = now - Duration::days(10);
    let past_end = now - Duration::days(5);
    let current_start = now - Duration::days(1);
    let current_end = now + Duration::days(1);
    let future_start = now + Duration::days(5);
    let future_end = now + Duration::days(10);

    // --- Past, current, and future roles for Alice ---
    kg.add_fact("Alice", "role", "Developer", past_start, past_end)
        .unwrap();
    kg.add_fact(
        "Alice",
        "role",
        "Lead Developer",
        current_start,
        current_end,
    )
    .unwrap();
    kg.add_fact("Alice", "role", "Architect", future_start, future_end)
        .unwrap();

    // --- Assertions ---

    // 1. Query for the current role
    let current_role = kg.get_fact_as_of("Alice", "role", now).unwrap();
    assert_eq!(
        current_role,
        Some("Lead Developer".to_string()),
        "Should retrieve the current role."
    );

    // 2. Query for a past role
    let past_role = kg
        .get_fact_as_of("Alice", "role", past_start + Duration::days(1))
        .unwrap();
    assert_eq!(
        past_role,
        Some("Developer".to_string()),
        "Should retrieve the past role."
    );

    // 3. Query for a future role
    let future_role = kg
        .get_fact_as_of("Alice", "role", future_start + Duration::days(1))
        .unwrap();
    assert_eq!(
        future_role,
        Some("Architect".to_string()),
        "Should retrieve the future role."
    );

    // 4. Query for a time where no role is assigned
    let no_role_time = now - Duration::days(3);
    let no_role = kg.get_fact_as_of("Alice", "role", no_role_time).unwrap();
    assert_eq!(no_role, None, "Should retrieve no role.");
}
