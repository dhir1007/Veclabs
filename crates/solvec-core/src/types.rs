use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single vector with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Query result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Distance metric options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}