//! # Knowledge Graph
//!
//! This module provides an interface for a knowledge graph, leveraging the `indradb`
//! library for graph storage and querying. It is designed to model facts with
//! time-based validity, allowing for queries that retrieve information "as of"
//! a specific moment. This entire module is compiled only when the `graph_db`
//! feature is enabled.

pub mod types;

use self::types::{
    KnowledgeGraph, KnowledgeGraphError, MemoryKnowledgeGraph, RocksdbKnowledgeGraph,
    TimeConstraint,
};
use chrono::{DateTime, Utc};
use indradb::{
    Datastore, Edge, Identifier, Json, MemoryDatastore, QueryExt, RocksdbDatastore,
    SpecificVertexQuery, Transaction, Vertex,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

const TIME_PROPERTY_NAME: &str = "time";
const NAME_PROPERTY_NAME: &str = "name";

impl MemoryKnowledgeGraph {
    /// Creates a new in-memory `KnowledgeGraph`.
    pub fn new_memory() -> Self {
        Self {
            db: MemoryDatastore::new_db(),
            entity_map: HashMap::new(),
        }
    }

    /// Clears all data from the graph and resets the local entity map.
    pub fn clear(&mut self) -> Result<(), KnowledgeGraphError> {
        self.db = MemoryDatastore::new_db();
        self.entity_map.clear();
        Ok(())
    }
}

impl RocksdbKnowledgeGraph {
    /// Creates a new `KnowledgeGraph` backed by a RocksDB datastore at the
    /// specified path.
    pub fn new_rocksdb<P: AsRef<Path>>(path: P) -> Result<Self, KnowledgeGraphError> {
        let datastore = RocksdbDatastore::new_db(path)?;
        Ok(Self {
            db: datastore,
            entity_map: HashMap::new(),
        })
    }
}

impl<D: Datastore> KnowledgeGraph<D> {
    /// Retrieves or creates a vertex for a given entity name, caching it locally.
    /// This function will "slugify" the name to create a valid vertex type identifier,
    /// and store the original name in a "name" property.
    fn get_or_create_vertex<'a>(
        entity_map: &mut HashMap<String, Uuid>,
        transaction: &mut <D as Datastore>::Transaction<'a>,
        name: &str,
    ) -> Result<Uuid, KnowledgeGraphError> {
        if let Some(id) = entity_map.get(name) {
            return Ok(*id);
        }

        let slug: String = name
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let vertex_type = Identifier::new(&slug)?;
        let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes());
        let vertex = Vertex::with_id(id, vertex_type);
        transaction.create_vertex(&vertex)?;

        // Store the original name as a property
        let name_prop = Identifier::new(NAME_PROPERTY_NAME)?;
        let name_json = Json::new(json!(name));
        transaction.set_vertex_properties(vec![vertex.id], name_prop, &name_json)?;

        entity_map.insert(name.to_string(), vertex.id);
        Ok(vertex.id)
    }

    /// Adds a fact (an edge) to the knowledge graph with a specified validity period.
    pub fn add_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<(), KnowledgeGraphError> {
        let mut transaction = self.db.datastore.transaction();
        let subject_id =
            Self::get_or_create_vertex(&mut self.entity_map, &mut transaction, subject)?;
        let object_id = Self::get_or_create_vertex(&mut self.entity_map, &mut transaction, object)?;
        let predicate_id = Identifier::new(predicate)?;

        let edge = Edge::new(subject_id, predicate_id, object_id);
        transaction.create_edge(&edge)?;

        let time_constraint = TimeConstraint {
            start_time,
            end_time,
        };
        let time_prop_name = Identifier::new(TIME_PROPERTY_NAME)?;

        transaction.set_edge_properties(
            vec![edge],
            time_prop_name,
            &Json::new(json!(time_constraint)),
        )?;

        // The transaction is automatically committed/rolled back when it goes
        // out of scope (RAII), as the `Transaction` trait does not define a
        // `commit` method. Returning Ok(()) ensures it commits.
        Ok(())
    }

    /// Retrieves the object of a fact that is valid at a specific point in time.
    pub fn get_fact_as_of(
        &self,
        subject: &str,
        predicate: &str,
        as_of: DateTime<Utc>,
    ) -> Result<Option<String>, KnowledgeGraphError> {
        let subject_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, subject.as_bytes());
        let predicate_id = Identifier::new(predicate)?;

        // Build a query that gets all properties for the outbound edges of a specific type from the subject.
        let query = SpecificVertexQuery::single(subject_id)
            .outbound()?
            .t(predicate_id)
            .properties()?;

        let results = self.db.get(query)?;
        let edge_properties =
            indradb::util::extract_edge_properties(results).ok_or(KnowledgeGraphError::NotFound)?;

        let time_prop_name = Identifier::new(TIME_PROPERTY_NAME)?;

        for prop in edge_properties {
            if let Some(time_json) = prop.props.iter().find(|p| p.name == time_prop_name) {
                let time_constraint: TimeConstraint =
                    serde_json::from_value((*time_json.value.0).clone())?;

                if as_of >= time_constraint.start_time && as_of < time_constraint.end_time {
                    // Found a valid edge. Now get the object's name property.
                    let object_id = prop.edge.inbound_id;
                    let name_prop = Identifier::new(NAME_PROPERTY_NAME)?;
                    let prop_query = SpecificVertexQuery::single(object_id)
                        .properties()?
                        .name(name_prop);

                    let prop_results = self.db.get(prop_query)?;
                    let vertex_props = indradb::util::extract_vertex_properties(prop_results)
                        .ok_or(KnowledgeGraphError::NotFound)?;

                    if let Some(v_prop) = vertex_props.into_iter().next() {
                        if let Some(named_prop) = v_prop.props.into_iter().next() {
                            if let serde_json::Value::String(s) = named_prop.value.0.as_ref() {
                                return Ok(Some(s.clone()));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}
