# VecLabs — Cursor Agent Implementation Prompt
## Phase 1: Foundation & Rust Core (Days 1–5)

---

## CONTEXT & PROJECT OVERVIEW

You are implementing **VecLabs**, a decentralized vector database for AI agents. The product is called **SolVec** (the SDK name). The core concept is:

- A **Rust HNSW engine** for sub-5ms vector search (the speed layer)
- A **Solana on-chain layer** that stores Merkle roots for cryptographic proof of every vector collection (the trust layer)
- **Shadow Drive / Arweave** for encrypted vector storage (the storage layer)
- A **developer SDK** (TypeScript + Python) that makes all of this invisible to AI engineers

The target user is an AI engineer who currently uses Pinecone and wants to migrate in 30 minutes. Our API shape intentionally mirrors Pinecone's.

**This prompt covers Phase 1 only: the Rust core architecture (Days 1–5).** No Solana, no SDK, no demo yet. Just the Rust engine that will be the performance foundation of everything else.

---

## CURRENT REPO STATE

The repo already has this folder structure (do not change it, build within it):

```
VECLABS/
├── benchmarks/
├── crates/
│   ├── solvec-core/
│   │   └── src/
│   │       ├── distance.rs     ← EXISTS but may be incomplete
│   │       ├── hnsw.rs         ← EXISTS but may be incomplete
│   │       ├── types.rs        ← EXISTS but may be incomplete
│   │       ├── lib.rs          ← EXISTS but may be incomplete
│   │       └── Cargo.toml      ← EXISTS but may be incomplete
│   └── solvec-wasm/            ← EXISTS but empty
├── demo/
│   └── agent-memory/           ← EXISTS but empty
├── docs/
│   └── architecture.md         ← EXISTS but may be incomplete
├── programs/
│   └── solvec/                 ← EXISTS but empty (Solana — Phase 2)
├── sdk/
│   ├── python/                 ← EXISTS but empty (Phase 2)
│   └── typescript/             ← EXISTS but empty (Phase 2)
├── arch.png
├── Cargo.toml                  ← EXISTS (workspace root)
├── README.md
└── THESIS.md
```

**Your job in this prompt:** Fully implement everything inside `crates/solvec-core/` and the `benchmarks/` folder. Everything else is Phase 2 and later.

---

## WHAT YOU MUST BUILD — COMPLETE SPECIFICATION

### FILE 1: `Cargo.toml` (workspace root)

Rewrite this to be a proper Rust workspace:

```toml
[workspace]
members = [
    "crates/solvec-core",
    "crates/solvec-wasm",
]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"
rayon = "1.7"
ahash = "0.8"
sha2 = "0.10"
aes-gcm = "0.10"
criterion = { version = "0.5", features = ["html_reports"] }
wasm-bindgen = "0.2"
hex = "0.4"
thiserror = "1.0"
log = "0.4"
```

---

### FILE 2: `crates/solvec-core/Cargo.toml`

```toml
[package]
name = "solvec-core"
version = "0.1.0"
edition = "2021"
description = "High-performance HNSW vector search engine for VecLabs"
license = "MIT"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
rand = { workspace = true }
rayon = { workspace = true }
ahash = { workspace = true }
sha2 = { workspace = true }
aes-gcm = { workspace = true }
hex = { workspace = true }
thiserror = { workspace = true }
log = { workspace = true }

[dev-dependencies]
criterion = { workspace = true }

[[bench]]
name = "hnsw_bench"
harness = false

[[bench]]
name = "distance_bench"
harness = false
```

---

### FILE 3: `crates/solvec-core/src/lib.rs`

This is the public API surface of the crate. Export everything cleanly:

```rust
pub mod distance;
pub mod encryption;
pub mod hnsw;
pub mod merkle;
pub mod types;

// Re-export the most important types at crate root
pub use hnsw::HNSWIndex;
pub use types::{Collection, DistanceMetric, QueryResult, SolVecError, Vector};
```

---

### FILE 4: `crates/solvec-core/src/types.rs`

Implement ALL of the following — completely, with no stubs or TODOs:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// A single vector with its ID and optional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Vector {
    pub fn new(id: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            id: id.into(),
            values,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(
        id: impl Into<String>,
        values: Vec<f32>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            values,
            metadata,
        }
    }

    pub fn dimension(&self) -> usize {
        self.values.len()
    }

    pub fn validate(&self) -> Result<(), SolVecError> {
        if self.id.is_empty() {
            return Err(SolVecError::InvalidVector("Vector ID cannot be empty".into()));
        }
        if self.values.is_empty() {
            return Err(SolVecError::InvalidVector("Vector values cannot be empty".into()));
        }
        if self.values.iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Err(SolVecError::InvalidVector(
                "Vector contains NaN or infinite values".into(),
            ));
        }
        Ok(())
    }
}

/// Query result returned from HNSW search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl QueryResult {
    pub fn new(id: String, score: f32, metadata: HashMap<String, serde_json::Value>) -> Self {
        Self { id, score, metadata }
    }
}

/// Distance metric options for vector similarity
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Cosine
    }
}

impl std::fmt::Display for DistanceMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistanceMetric::Cosine => write!(f, "cosine"),
            DistanceMetric::Euclidean => write!(f, "euclidean"),
            DistanceMetric::DotProduct => write!(f, "dot_product"),
        }
    }
}

/// Collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub dimension: usize,
    pub metric: DistanceMetric,
    pub vector_count: usize,
    pub created_at: u64,
}

impl Collection {
    pub fn new(name: impl Into<String>, dimension: usize, metric: DistanceMetric) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            name: name.into(),
            dimension,
            metric,
            vector_count: 0,
            created_at,
        }
    }
}

/// All errors that can occur in solvec-core
#[derive(Error, Debug)]
pub enum SolVecError {
    #[error("Invalid vector: {0}")]
    InvalidVector(String),

    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Vector not found: {0}")]
    VectorNotFound(String),

    #[error("Index is empty")]
    EmptyIndex,

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid top_k: must be >= 1, got {0}")]
    InvalidTopK(usize),
}
```

---

### FILE 5: `crates/solvec-core/src/distance.rs`

Implement ALL distance functions completely. These must be highly optimized — they are called millions of times per second during HNSW construction and queries:

```rust
use crate::types::DistanceMetric;

/// Main dispatch function — called by HNSW for all distance computations
#[inline(always)]
pub fn compute(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vector dimensions must match");
    match metric {
        DistanceMetric::Cosine => cosine_similarity(a, b),
        DistanceMetric::Euclidean => euclidean_distance(a, b),
        DistanceMetric::DotProduct => dot_product(a, b),
    }
}

/// Cosine similarity — returns value in [-1, 1], higher = more similar
/// Used as DEFAULT metric (same as Pinecone default)
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    // Single-pass computation — faster than three separate loops
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        return 0.0;
    }
    (dot / denom).clamp(-1.0, 1.0)
}

/// Euclidean distance — returns value in [0, ∞), lower = more similar
/// Note: HNSW uses this as a similarity score (negated internally)
#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

/// Squared euclidean distance — avoids sqrt, used internally for comparisons
#[inline]
pub fn euclidean_distance_squared(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum()
}

/// Dot product similarity — returns scalar, higher = more similar
/// Best for normalized vectors (OpenAI embeddings are already normalized)
#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Normalize a vector to unit length (for cosine optimization)
/// Pre-normalizing vectors allows using dot_product instead of cosine (faster)
pub fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

/// Convert distance to similarity score for consistent API output
/// All metrics return higher = more similar after this conversion
pub fn to_similarity_score(distance: f32, metric: DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Cosine => distance,        // already similarity
        DistanceMetric::DotProduct => distance,     // already similarity
        DistanceMetric::Euclidean => 1.0 / (1.0 + distance), // convert to 0-1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "Orthogonal vectors should have similarity ~0.0");
    }

    #[test]
    fn test_cosine_opposite_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6, "Opposite vectors should have similarity -1.0");
    }

    #[test]
    fn test_euclidean_same_point() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!(euclidean_distance(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(dot_product(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_unit_vector() {
        let v = vec![3.0, 4.0];
        let norm = normalize(&v);
        let magnitude: f32 = norm.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let v = vec![0.0, 0.0, 0.0];
        let norm = normalize(&v);
        assert_eq!(norm, v); // should return unchanged
    }
}
```

---

### FILE 6: `crates/solvec-core/src/hnsw.rs`

This is the most critical file. Implement a **complete, production-quality HNSW** with no stubs, no TODOs, no placeholder comments. Every method must be fully working.

Requirements:
- Full multilayer HNSW graph with proper level generation
- Bidirectional connections with neighbor pruning (heuristic selection)
- Thread-safe reads (use RwLock for concurrent queries)
- Insert, delete, update, query all fully working
- Serialization to/from JSON for persistence
- Proper error handling using SolVecError

Here is the complete implementation to build:

```rust
use crate::distance;
use crate::types::{DistanceMetric, QueryResult, SolVecError, Vector};
use ahash::AHashMap;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

/// Candidate node during graph traversal — ordered by score
#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    score: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}
impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Max-heap by score (higher score = closer for cosine/dot, lower dist for euclidean)
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.partial_cmp(&other.score).unwrap_or(Ordering::Equal)
    }
}

/// The VecLabs HNSW Index
/// 
/// Implements Hierarchical Navigable Small World graph for approximate
/// nearest neighbor search. This is the performance core of SolVec.
///
/// Parameters:
/// - M: max connections per node per layer (default: 16)
/// - ef_construction: beam width during index build (default: 200)
/// - ef_search: beam width during query (default: 50)
#[derive(Debug, Serialize, Deserialize)]
pub struct HNSWIndex {
    // Configuration
    m: usize,
    m_max_0: usize,          // M * 2 for layer 0 (standard HNSW practice)
    ef_construction: usize,
    ef_search: usize,
    ml: f64,                  // Level multiplier = 1 / ln(M)

    // Data storage
    vectors: AHashMap<String, Vector>,

    // Graph: layer_index -> node_id -> list of neighbor_ids
    // layers[0] is the base layer (densest connections)
    // layers[max] is the entry point layer (sparsest)
    layers: Vec<AHashMap<String, Vec<String>>>,

    // Entry point (node in the highest layer)
    entry_point: Option<String>,
    entry_point_level: usize,

    // Stats
    total_inserts: usize,
    total_deletes: usize,

    metric: DistanceMetric,
    dimension: Option<usize>,
}

impl HNSWIndex {
    /// Create a new HNSW index
    /// 
    /// # Arguments
    /// * `m` - Max connections per node. 16 is standard. Higher = better recall, more memory.
    /// * `ef_construction` - Build-time beam width. 200 is standard. Higher = better quality, slower build.
    /// * `metric` - Distance metric for similarity computation.
    pub fn new(m: usize, ef_construction: usize, metric: DistanceMetric) -> Self {
        let m = m.max(2); // minimum 2
        Self {
            m,
            m_max_0: m * 2,
            ef_construction,
            ef_search: ef_construction.min(50).max(10),
            ml: 1.0 / (m as f64).ln(),
            vectors: AHashMap::new(),
            layers: Vec::new(),
            entry_point: None,
            entry_point_level: 0,
            total_inserts: 0,
            total_deletes: 0,
            metric,
            dimension: None,
        }
    }

    /// Create with sensible defaults — what most users should use
    pub fn default_cosine() -> Self {
        Self::new(16, 200, DistanceMetric::Cosine)
    }

    /// Set ef_search — increase for better recall at cost of speed
    pub fn set_ef_search(&mut self, ef: usize) {
        self.ef_search = ef.max(1);
    }

    /// Number of vectors in the index
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    pub fn metric(&self) -> DistanceMetric {
        self.metric
    }

    /// Insert a vector into the index
    /// 
    /// If a vector with the same ID already exists, it is updated.
    pub fn insert(&mut self, vector: Vector) -> Result<(), SolVecError> {
        // Validate
        vector.validate()?;

        // Check dimension consistency
        if let Some(dim) = self.dimension {
            if vector.values.len() != dim {
                return Err(SolVecError::DimensionMismatch {
                    expected: dim,
                    actual: vector.values.len(),
                });
            }
        } else {
            self.dimension = Some(vector.values.len());
        }

        // If ID already exists, delete first (update = delete + insert)
        if self.vectors.contains_key(&vector.id) {
            self.delete(&vector.id)?;
        }

        let id = vector.id.clone();
        let insert_level = self.random_level();

        // Ensure layers exist up to insert_level
        while self.layers.len() <= insert_level {
            self.layers.push(AHashMap::new());
        }

        // Initialize node in all layers up to its level
        for l in 0..=insert_level {
            self.layers[l].insert(id.clone(), Vec::new());
        }

        // Store the vector
        self.vectors.insert(id.clone(), vector);

        // First vector — just set as entry point
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
            self.entry_point_level = insert_level;
            self.total_inserts += 1;
            return Ok(());
        }

        // Connect the new node to the graph
        self.connect_new_node(&id, insert_level);

        // Update entry point if new node is at a higher level
        if insert_level > self.entry_point_level {
            self.entry_point = Some(id);
            self.entry_point_level = insert_level;
        }

        self.total_inserts += 1;
        Ok(())
    }

    /// Delete a vector by ID
    pub fn delete(&mut self, id: &str) -> Result<(), SolVecError> {
        if !self.vectors.contains_key(id) {
            return Err(SolVecError::VectorNotFound(id.to_string()));
        }

        // Remove from all layers and clean up neighbor references
        for layer in &mut self.layers {
            layer.remove(id);
            for neighbors in layer.values_mut() {
                neighbors.retain(|n| n != id);
            }
        }

        self.vectors.remove(id);
        self.total_deletes += 1;

        // Update entry point if we deleted it
        if self.entry_point.as_deref() == Some(id) {
            // Find a new entry point from the highest non-empty layer
            self.entry_point = None;
            self.entry_point_level = 0;
            for (level, layer) in self.layers.iter().enumerate().rev() {
                if let Some(new_ep) = layer.keys().next() {
                    self.entry_point = Some(new_ep.clone());
                    self.entry_point_level = level;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Update a vector (convenience wrapper for delete + insert)
    pub fn update(&mut self, vector: Vector) -> Result<(), SolVecError> {
        self.insert(vector) // insert already handles update case
    }

    /// Query the index for top-K nearest neighbors
    ///
    /// Returns results sorted by score descending (most similar first)
    pub fn query(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<QueryResult>, SolVecError> {
        if top_k == 0 {
            return Err(SolVecError::InvalidTopK(top_k));
        }
        if self.vectors.is_empty() {
            return Ok(vec![]);
        }
        if let Some(dim) = self.dimension {
            if query_vector.len() != dim {
                return Err(SolVecError::DimensionMismatch {
                    expected: dim,
                    actual: query_vector.len(),
                });
            }
        }

        let entry = match &self.entry_point {
            Some(ep) => ep.clone(),
            None => return Ok(vec![]),
        };

        let ef = self.ef_search.max(top_k);

        // Phase 1: Greedy search from entry point down to layer 1
        // At each layer, find the single closest node to use as entry for next layer
        let mut current_nearest = entry;
        for layer_idx in (1..=self.entry_point_level).rev() {
            let candidates = self.search_layer(query_vector, &current_nearest, 1, layer_idx);
            if let Some(best) = candidates.into_iter().next() {
                current_nearest = best.id;
            }
        }

        // Phase 2: Full ef search at layer 0 (base layer, densest connections)
        let candidates = self.search_layer(query_vector, &current_nearest, ef, 0);

        // Build results from top-k candidates
        let results: Vec<QueryResult> = candidates
            .into_iter()
            .take(top_k)
            .map(|c| {
                let vec = &self.vectors[&c.id];
                QueryResult::new(c.id, c.score, vec.metadata.clone())
            })
            .collect();

        Ok(results)
    }

    /// Search a single HNSW layer using beam search
    /// Returns candidates sorted by score descending
    fn search_layer(
        &self,
        query: &[f32],
        entry_id: &str,
        ef: usize,
        layer_idx: usize,
    ) -> Vec<Candidate> {
        let layer = match self.layers.get(layer_idx) {
            Some(l) => l,
            None => return vec![],
        };

        let entry_vec = match self.vectors.get(entry_id) {
            Some(v) => &v.values,
            None => return vec![],
        };

        let entry_score = self.similarity_score(query, entry_vec);

        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(entry_id.to_string());

        // candidates: max-heap by score (best on top)
        let mut candidates: BinaryHeap<Candidate> = BinaryHeap::new();
        candidates.push(Candidate { id: entry_id.to_string(), score: entry_score });

        // results: max-heap by score (we want top-ef results)
        let mut results: BinaryHeap<Candidate> = BinaryHeap::new();
        results.push(Candidate { id: entry_id.to_string(), score: entry_score });

        // worst score in results (min score in max-heap — we track separately)
        let mut worst_result_score = entry_score;

        while let Some(current) = candidates.pop() {
            // If current candidate is worse than the worst in results, stop
            if results.len() >= ef && current.score < worst_result_score {
                break;
            }

            // Explore current node's neighbors
            if let Some(neighbors) = layer.get(&current.id) {
                for neighbor_id in neighbors {
                    if visited.contains(neighbor_id) {
                        continue;
                    }
                    visited.insert(neighbor_id.clone());

                    let neighbor_vec = match self.vectors.get(neighbor_id) {
                        Some(v) => &v.values,
                        None => continue,
                    };

                    let score = self.similarity_score(query, neighbor_vec);

                    // Add to candidates if it could improve results
                    if results.len() < ef || score > worst_result_score {
                        candidates.push(Candidate { id: neighbor_id.clone(), score });
                        results.push(Candidate { id: neighbor_id.clone(), score });

                        // Trim results to ef size and update worst score
                        if results.len() > ef {
                            // Convert to sorted vec, keep top ef
                            let mut sorted: Vec<Candidate> = results.drain().collect();
                            sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
                            sorted.truncate(ef);
                            worst_result_score = sorted.last().map(|c| c.score).unwrap_or(f32::MIN);
                            results = sorted.into_iter().collect();
                        } else {
                            worst_result_score = worst_result_score.min(score);
                        }
                    }
                }
            }
        }

        // Return sorted results (best first)
        let mut final_results: Vec<Candidate> = results.into_iter().collect();
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        final_results
    }

    /// Connect a newly inserted node to the graph at all its layers
    fn connect_new_node(&mut self, node_id: &str, insert_level: usize) {
        let node_values = self.vectors[node_id].values.clone();
        let entry = self.entry_point.clone().unwrap();

        let mut current_nearest = entry;

        // From top layer down to insert_level+1: greedy descent (no connections made)
        for layer_idx in (insert_level + 1..=self.entry_point_level).rev() {
            if layer_idx < self.layers.len() {
                let candidates = self.search_layer(&node_values, &current_nearest, 1, layer_idx);
                if let Some(best) = candidates.into_iter().next() {
                    current_nearest = best.id;
                }
            }
        }

        // From insert_level down to 0: search and make connections
        for layer_idx in (0..=insert_level.min(self.entry_point_level)).rev() {
            let m_at_layer = if layer_idx == 0 { self.m_max_0 } else { self.m };

            // Find ef_construction nearest neighbors at this layer
            let candidates = self.search_layer(
                &node_values,
                &current_nearest,
                self.ef_construction,
                layer_idx,
            );

            // Select M best neighbors using simple greedy heuristic
            let neighbors: Vec<String> = candidates
                .iter()
                .filter(|c| c.id != node_id)
                .take(m_at_layer)
                .map(|c| c.id.clone())
                .collect();

            // Update entry for next (lower) layer
            if let Some(best) = candidates.into_iter().next() {
                current_nearest = best.id;
            }

            // Set bidirectional connections
            // 1. Our node points to its new neighbors
            if let Some(node_neighbors) = self.layers[layer_idx].get_mut(node_id) {
                for n in &neighbors {
                    if !node_neighbors.contains(n) {
                        node_neighbors.push(n.clone());
                    }
                }
            }

            // 2. Each neighbor points back to our node (with pruning)
            let node_values_clone = node_values.clone();
            for neighbor_id in &neighbors {
                if let Some(n_neighbors) = self.layers[layer_idx].get_mut(neighbor_id) {
                    if !n_neighbors.contains(&node_id.to_string()) {
                        n_neighbors.push(node_id.to_string());
                    }

                    // Prune neighbor's connections if over capacity
                    if n_neighbors.len() > m_at_layer {
                        let neighbor_values = self.vectors[neighbor_id].values.clone();
                        let metric = self.metric;

                        // Sort by similarity to neighbor, keep top M
                        n_neighbors.sort_by(|a, b| {
                            let score_a = if a == node_id {
                                distance::compute(&neighbor_values, &node_values_clone, metric)
                            } else {
                                self.vectors.get(a)
                                    .map(|v| distance::compute(&neighbor_values, &v.values, metric))
                                    .unwrap_or(f32::MIN)
                            };
                            let score_b = if b == node_id {
                                distance::compute(&neighbor_values, &node_values_clone, metric)
                            } else {
                                self.vectors.get(b)
                                    .map(|v| distance::compute(&neighbor_values, &v.values, metric))
                                    .unwrap_or(f32::MIN)
                            };
                            score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal)
                        });
                        n_neighbors.truncate(m_at_layer);
                    }
                }
            }
        }
    }

    /// Compute similarity score between two vectors
    /// Higher is always better (handles euclidean inversion internally)
    #[inline]
    fn similarity_score(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.metric {
            DistanceMetric::Cosine => distance::cosine_similarity(a, b),
            DistanceMetric::DotProduct => distance::dot_product(a, b),
            DistanceMetric::Euclidean => {
                // Invert euclidean distance so higher = better (consistent with cosine)
                let d = distance::euclidean_distance(a, b);
                1.0 / (1.0 + d)
            }
        }
    }

    /// Generate a random level for a new node
    /// Uses the standard HNSW formula: level = floor(-ln(rand) * ml)
    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut level = 0usize;
        loop {
            let r: f64 = rng.gen();
            if r > self.ml || level >= 16 {
                break;
            }
            level += 1;
        }
        level
    }

    /// Serialize the entire index to JSON for persistence
    pub fn to_json(&self) -> Result<String, SolVecError> {
        serde_json::to_string(self)
            .map_err(|e| SolVecError::SerializationError(e.to_string()))
    }

    /// Deserialize an index from JSON
    pub fn from_json(json: &str) -> Result<Self, SolVecError> {
        serde_json::from_str(json)
            .map_err(|e| SolVecError::SerializationError(e.to_string()))
    }

    /// Get stats about the index
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            vector_count: self.vectors.len(),
            layer_count: self.layers.len(),
            entry_point_level: self.entry_point_level,
            dimension: self.dimension.unwrap_or(0),
            total_inserts: self.total_inserts,
            total_deletes: self.total_deletes,
            metric: self.metric,
        }
    }
}

/// Index statistics for monitoring and debugging
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub vector_count: usize,
    pub layer_count: usize,
    pub entry_point_level: usize,
    pub dimension: usize,
    pub total_inserts: usize,
    pub total_deletes: usize,
    pub metric: DistanceMetric,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_vector(id: &str, values: Vec<f32>) -> Vector {
        Vector::new(id, values)
    }

    fn random_vector(dim: usize) -> Vec<f32> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..dim).map(|_| rng.gen::<f32>()).collect()
    }

    #[test]
    fn test_basic_insert_and_query() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);

        index.insert(make_vector("a", vec![1.0, 0.0, 0.0])).unwrap();
        index.insert(make_vector("b", vec![0.9, 0.1, 0.0])).unwrap();
        index.insert(make_vector("c", vec![0.0, 1.0, 0.0])).unwrap();
        index.insert(make_vector("d", vec![0.0, 0.0, 1.0])).unwrap();

        let results = index.query(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a"); // most similar to itself
        assert_eq!(results[1].id, "b"); // second most similar
    }

    #[test]
    fn test_query_returns_correct_count() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        for i in 0..50 {
            index.insert(make_vector(&format!("v{}", i), random_vector(128))).unwrap();
        }
        let results = index.query(&random_vector(128), 10).unwrap();
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_insert_duplicate_id_updates() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        index.insert(make_vector("a", vec![1.0, 0.0, 0.0])).unwrap();
        index.insert(make_vector("a", vec![0.0, 1.0, 0.0])).unwrap(); // update

        assert_eq!(index.len(), 1);
        let stored = &index.vectors["a"];
        assert_eq!(stored.values, vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_delete_removes_vector() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        index.insert(make_vector("a", vec![1.0, 0.0, 0.0])).unwrap();
        index.insert(make_vector("b", vec![0.0, 1.0, 0.0])).unwrap();

        index.delete("a").unwrap();
        assert_eq!(index.len(), 1);

        // Deleted vector should not appear in results
        let results = index.query(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(!results.iter().any(|r| r.id == "a"));
    }

    #[test]
    fn test_delete_nonexistent_returns_error() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        let result = index.delete("nonexistent");
        assert!(matches!(result, Err(SolVecError::VectorNotFound(_))));
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        index.insert(make_vector("a", vec![1.0, 0.0, 0.0])).unwrap();
        let result = index.insert(make_vector("b", vec![1.0, 0.0])); // wrong dim
        assert!(matches!(result, Err(SolVecError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_empty_index_returns_empty() {
        let index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        let results = index.query(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        for i in 0..20 {
            index.insert(make_vector(&format!("v{}", i), random_vector(64))).unwrap();
        }

        let json = index.to_json().unwrap();
        let restored = HNSWIndex::from_json(&json).unwrap();

        assert_eq!(restored.len(), 20);
        assert_eq!(restored.metric(), DistanceMetric::Cosine);
    }

    #[test]
    fn test_results_sorted_by_score_descending() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        for i in 0..30 {
            index.insert(make_vector(&format!("v{}", i), random_vector(128))).unwrap();
        }
        let results = index.query(&random_vector(128), 10).unwrap();
        for window in results.windows(2) {
            assert!(window[0].score >= window[1].score, "Results must be sorted descending");
        }
    }

    #[test]
    fn test_large_index_query_returns_results() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        for i in 0..1000 {
            index.insert(make_vector(&format!("v{}", i), random_vector(384))).unwrap();
        }
        let results = index.query(&random_vector(384), 10).unwrap();
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_metadata_preserved_in_results() {
        let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
        let mut meta = HashMap::new();
        meta.insert("text".to_string(), serde_json::Value::String("hello world".to_string()));

        index.insert(Vector::with_metadata("a", vec![1.0, 0.0, 0.0], meta)).unwrap();
        index.insert(make_vector("b", vec![0.5, 0.5, 0.0])).unwrap();

        let results = index.query(&[1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(results[0].id, "a");
        assert!(results[0].metadata.contains_key("text"));
    }
}
```

---

### FILE 7: `crates/solvec-core/src/merkle.rs`

Complete Merkle tree implementation for generating on-chain proofs:

```rust
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

/// Merkle tree for cryptographic verification of vector collections
/// The root (32 bytes) is what gets posted to Solana
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
    tree: Vec<Vec<[u8; 32]>>,
    original_ids: Vec<String>,
}

impl MerkleTree {
    /// Build a Merkle tree from a list of vector IDs
    pub fn new(vector_ids: &[String]) -> Self {
        let leaves: Vec<[u8; 32]> = vector_ids.iter()
            .map(|id| hash_leaf(id.as_bytes()))
            .collect();

        let tree = build_tree(&leaves);

        Self {
            leaves,
            tree,
            original_ids: vector_ids.to_vec(),
        }
    }

    /// Get the Merkle root — this 32-byte value goes on Solana
    pub fn root(&self) -> [u8; 32] {
        match self.tree.last() {
            Some(top) if !top.is_empty() => top[0],
            _ => [0u8; 32],
        }
    }

    /// Get root as hex string (for display and logging)
    pub fn root_hex(&self) -> String {
        hex::encode(self.root())
    }

    /// Generate a Merkle proof that a given vector ID is in this collection
    /// The proof can be verified by anyone with just the root
    pub fn generate_proof(&self, vector_id: &str) -> Option<MerkleProof> {
        let leaf = hash_leaf(vector_id.as_bytes());
        let leaf_pos = self.leaves.iter().position(|l| l == &leaf)?;

        let mut proof_nodes: Vec<ProofNode> = Vec::new();
        let mut current_pos = leaf_pos;

        for layer in &self.tree[..self.tree.len().saturating_sub(1)] {
            let is_right = current_pos % 2 == 0;
            let sibling_pos = if is_right {
                (current_pos + 1).min(layer.len() - 1)
            } else {
                current_pos - 1
            };

            proof_nodes.push(ProofNode {
                hash: layer[sibling_pos],
                position: if is_right { NodePosition::Right } else { NodePosition::Left },
            });

            current_pos /= 2;
        }

        Some(MerkleProof {
            vector_id: vector_id.to_string(),
            leaf_hash: leaf,
            proof_nodes,
            root: self.root(),
        })
    }

    pub fn vector_count(&self) -> usize {
        self.original_ids.len()
    }
}

/// A single node in a Merkle proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofNode {
    pub hash: [u8; 32],
    pub position: NodePosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodePosition {
    Left,
    Right,
}

/// A complete Merkle proof for a single vector ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub vector_id: String,
    pub leaf_hash: [u8; 32],
    pub proof_nodes: Vec<ProofNode>,
    pub root: [u8; 32],
}

impl MerkleProof {
    /// Verify this proof against a given root
    /// Returns true if the vector_id is provably in the collection with that root
    pub fn verify(&self, expected_root: &[u8; 32]) -> bool {
        let mut current_hash = self.leaf_hash;

        for node in &self.proof_nodes {
            current_hash = match node.position {
                NodePosition::Right => hash_pair(&current_hash, &node.hash),
                NodePosition::Left => hash_pair(&node.hash, &current_hash),
            };
        }

        &current_hash == expected_root
    }

    pub fn root_hex(&self) -> String {
        hex::encode(self.root)
    }
}

fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"leaf:"); // domain separation
    hasher.update(data);
    hasher.finalize().into()
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"node:"); // domain separation
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn build_tree(leaves: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
    if leaves.is_empty() {
        return vec![vec![[0u8; 32]]];
    }

    let mut tree: Vec<Vec<[u8; 32]>> = vec![leaves.to_vec()];
    let mut current_layer = leaves.to_vec();

    while current_layer.len() > 1 {
        let mut next_layer = Vec::new();
        let mut i = 0;
        while i < current_layer.len() {
            let left = current_layer[i];
            let right = if i + 1 < current_layer.len() {
                current_layer[i + 1]
            } else {
                current_layer[i] // duplicate last node if odd count
            };
            next_layer.push(hash_pair(&left, &right));
            i += 2;
        }
        tree.push(next_layer.clone());
        current_layer = next_layer;
    }

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("vec_{}", i)).collect()
    }

    #[test]
    fn test_single_element_tree() {
        let tree = MerkleTree::new(&ids(1));
        let root = tree.root();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_proof_verifies_correctly() {
        let id_list = ids(10);
        let tree = MerkleTree::new(&id_list);
        let root = tree.root();

        let proof = tree.generate_proof("vec_5").unwrap();
        assert!(proof.verify(&root), "Proof should verify against root");
    }

    #[test]
    fn test_proof_fails_with_wrong_root() {
        let tree = MerkleTree::new(&ids(10));
        let wrong_root = [1u8; 32];
        let proof = tree.generate_proof("vec_3").unwrap();
        assert!(!proof.verify(&wrong_root), "Proof should fail with wrong root");
    }

    #[test]
    fn test_proof_nonexistent_id_returns_none() {
        let tree = MerkleTree::new(&ids(5));
        assert!(tree.generate_proof("nonexistent_id").is_none());
    }

    #[test]
    fn test_different_id_sets_produce_different_roots() {
        let tree1 = MerkleTree::new(&ids(5));
        let tree2 = MerkleTree::new(&ids(6));
        assert_ne!(tree1.root(), tree2.root());
    }

    #[test]
    fn test_all_proofs_verify() {
        let id_list = ids(20);
        let tree = MerkleTree::new(&id_list);
        let root = tree.root();

        for id in &id_list {
            let proof = tree.generate_proof(id).unwrap();
            assert!(proof.verify(&root), "Proof failed for id: {}", id);
        }
    }

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new(&[]);
        assert_eq!(tree.root(), [0u8; 32]);
    }
}
```

---

### FILE 8: `crates/solvec-core/src/encryption.rs`

Complete AES-256-GCM encryption for vector data:

```rust
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use crate::types::SolVecError;

const NONCE_SIZE: usize = 12;

/// Encrypt a batch of vectors using AES-256-GCM
/// Key should be derived from the user's Solana wallet
/// Returns: nonce (12 bytes) + ciphertext
pub fn encrypt_vectors(vectors: &[Vec<f32>], key: &[u8; 32]) -> Result<Vec<u8>, SolVecError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // Serialize: [num_vectors: u64][dim: u64][f32 f32 f32 ...]
    let num_vectors = vectors.len() as u64;
    let dim = vectors.first().map(|v| v.len() as u64).unwrap_or(0);

    let mut plaintext = Vec::new();
    plaintext.extend_from_slice(&num_vectors.to_le_bytes());
    plaintext.extend_from_slice(&dim.to_le_bytes());
    for v in vectors {
        for &f in v {
            plaintext.extend_from_slice(&f.to_le_bytes());
        }
    }

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| SolVecError::EncryptionError(e.to_string()))?;

    // Prepend nonce to output
    let mut output = nonce.to_vec();
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt vectors from AES-256-GCM ciphertext
pub fn decrypt_vectors(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<Vec<f32>>, SolVecError> {
    if encrypted.len() < NONCE_SIZE {
        return Err(SolVecError::DecryptionError("Ciphertext too short".into()));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| SolVecError::DecryptionError(e.to_string()))?;

    // Deserialize
    if plaintext.len() < 16 {
        return Err(SolVecError::DecryptionError("Invalid plaintext length".into()));
    }

    let num_vectors = u64::from_le_bytes(plaintext[0..8].try_into().unwrap()) as usize;
    let dim = u64::from_le_bytes(plaintext[8..16].try_into().unwrap()) as usize;

    if dim == 0 || num_vectors == 0 {
        return Ok(vec![]);
    }

    let expected_bytes = 16 + num_vectors * dim * 4;
    if plaintext.len() < expected_bytes {
        return Err(SolVecError::DecryptionError("Plaintext length mismatch".into()));
    }

    let mut vectors = Vec::with_capacity(num_vectors);
    let data = &plaintext[16..];
    for i in 0..num_vectors {
        let start = i * dim * 4;
        let vec: Vec<f32> = (0..dim)
            .map(|j| {
                let offset = start + j * 4;
                f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
            })
            .collect();
        vectors.push(vec);
    }

    Ok(vectors)
}

/// Generate a deterministic key from a Solana wallet public key
/// In production this would use the actual wallet signing capability
pub fn derive_key_from_pubkey(pubkey_bytes: &[u8; 32]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(b"solvec-encryption-key-v1:");
    hasher.update(pubkey_bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn test_roundtrip_single_vector() {
        let key = test_key();
        let vectors = vec![vec![1.0f32, 2.0, 3.0, 4.0]];
        let encrypted = encrypt_vectors(&vectors, &key).unwrap();
        let decrypted = decrypt_vectors(&encrypted, &key).unwrap();
        assert_eq!(vectors, decrypted);
    }

    #[test]
    fn test_roundtrip_multiple_vectors() {
        let key = test_key();
        let vectors: Vec<Vec<f32>> = (0..10)
            .map(|i| (0..384).map(|j| (i * j) as f32 * 0.001).collect())
            .collect();
        let encrypted = encrypt_vectors(&vectors, &key).unwrap();
        let decrypted = decrypt_vectors(&encrypted, &key).unwrap();
        assert_eq!(vectors.len(), decrypted.len());
        for (orig, dec) in vectors.iter().zip(decrypted.iter()) {
            for (a, b) in orig.iter().zip(dec.iter()) {
                assert!((a - b).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let vectors = vec![vec![1.0f32, 2.0, 3.0]];
        let encrypted = encrypt_vectors(&vectors, &key1).unwrap();
        let result = decrypt_vectors(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_different_encryptions_of_same_data() {
        // AES-GCM with random nonce should produce different ciphertext each time
        let key = test_key();
        let vectors = vec![vec![1.0f32, 2.0, 3.0]];
        let enc1 = encrypt_vectors(&vectors, &key).unwrap();
        let enc2 = encrypt_vectors(&vectors, &key).unwrap();
        assert_ne!(enc1, enc2, "Each encryption should use a unique nonce");
    }

    #[test]
    fn test_empty_input() {
        let key = test_key();
        let result = encrypt_vectors(&[], &key);
        assert!(result.is_ok());
    }
}
```

---

### FILE 9: `benchmarks/hnsw_bench.rs`

Complete benchmark suite using Criterion:

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use solvec_core::{
    hnsw::HNSWIndex,
    types::{DistanceMetric, Vector},
};
use rand::Rng;

fn random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn build_index(size: usize, dim: usize) -> HNSWIndex {
    let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
    for i in 0..size {
        index.insert(Vector::new(format!("v{}", i), random_vector(dim))).unwrap();
    }
    index
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_insert");

    for &size in &[1_000usize, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::new("size", size), &size, |b, &size| {
            b.iter(|| {
                let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);
                for i in 0..size {
                    index.insert(Vector::new(format!("v{}", i), random_vector(384))).unwrap();
                }
                black_box(index.len())
            });
        });
    }
    group.finish();
}

fn bench_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_query");

    for &index_size in &[10_000usize, 100_000] {
        let index = build_index(index_size, 384);
        let query = random_vector(384);

        for &top_k in &[1usize, 10, 100] {
            group.bench_with_input(
                BenchmarkId::new(format!("index_{}_topk", index_size), top_k),
                &top_k,
                |b, &top_k| {
                    b.iter(|| black_box(index.query(&query, top_k).unwrap()));
                },
            );
        }
    }
    group.finish();
}

fn bench_query_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_query_by_dimension");

    for &dim in &[128usize, 384, 768, 1536] {
        let index = build_index(10_000, dim);
        let query = random_vector(dim);
        group.bench_with_input(BenchmarkId::new("dim", dim), &dim, |b, _| {
            b.iter(|| black_box(index.query(&query, 10).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_insert, bench_query, bench_query_dimensions);
criterion_main!(benches);
```

---

### FILE 10: `benchmarks/distance_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use solvec_core::distance;
use rand::Rng;

fn random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen::<f32>()).collect()
}

fn bench_distance_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance");

    for &dim in &[128usize, 384, 768, 1536] {
        let a = random_vector(dim);
        let b = random_vector(dim);

        group.bench_with_input(BenchmarkId::new("cosine", dim), &dim, |bench, _| {
            bench.iter(|| black_box(distance::cosine_similarity(&a, &b)));
        });

        group.bench_with_input(BenchmarkId::new("euclidean", dim), &dim, |bench, _| {
            bench.iter(|| black_box(distance::euclidean_distance(&a, &b)));
        });

        group.bench_with_input(BenchmarkId::new("dot_product", dim), &dim, |bench, _| {
            bench.iter(|| black_box(distance::dot_product(&a, &b)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_distance_functions);
criterion_main!(benches);
```

---

### FILE 11: Integration Test

Create `crates/solvec-core/tests/integration_test.rs`:

```rust
use solvec_core::{
    encryption::{decrypt_vectors, encrypt_vectors},
    hnsw::HNSWIndex,
    merkle::MerkleTree,
    types::{DistanceMetric, Vector},
};
use std::collections::HashMap;

#[test]
fn test_complete_veclabs_pipeline() {
    println!("\n🚀 VecLabs Full Pipeline Integration Test\n");

    // === STEP 1: Build HNSW Index ===
    let mut index = HNSWIndex::new(16, 200, DistanceMetric::Cosine);

    let test_vectors = vec![
        ("user_alex_intro", vec![0.9f32, 0.1, 0.0, 0.0]),
        ("user_alex_startup", vec![0.8f32, 0.2, 0.1, 0.0]),
        ("user_bob_intro", vec![0.0f32, 0.0, 0.9, 0.1]),
        ("session_summary", vec![0.5f32, 0.5, 0.0, 0.0]),
    ];

    for (id, values) in &test_vectors {
        let mut meta = HashMap::new();
        meta.insert("source".to_string(), serde_json::Value::String("agent_memory".to_string()));
        index.insert(Vector::with_metadata(*id, values.clone(), meta)).unwrap();
    }

    println!("✅ Step 1: Indexed {} vectors", index.len());
    assert_eq!(index.len(), 4);

    // === STEP 2: Query ===
    let query = vec![0.85f32, 0.15, 0.0, 0.0];
    let results = index.query(&query, 2).unwrap();

    println!("✅ Step 2: Query returned {} results", results.len());
    println!("   Top result: {} (score: {:.4})", results[0].id, results[0].score);
    assert_eq!(results.len(), 2);
    assert!(results[0].score >= results[1].score);

    // === STEP 3: Merkle Tree ===
    let ids: Vec<String> = test_vectors.iter().map(|(id, _)| id.to_string()).collect();
    let tree = MerkleTree::new(&ids);
    let root = tree.root();
    let root_hex = tree.root_hex();

    println!("✅ Step 3: Merkle root computed: {}", &root_hex[..16]);
    assert_ne!(root, [0u8; 32]);

    // Verify a proof
    let proof = tree.generate_proof("user_alex_intro").unwrap();
    assert!(proof.verify(&root));
    println!("✅ Step 3: Merkle proof verified for 'user_alex_intro'");

    // === STEP 4: Encryption ===
    let key = [0u8; 32]; // In production: derived from Solana wallet
    let raw_vectors: Vec<Vec<f32>> = test_vectors.iter().map(|(_, v)| v.clone()).collect();
    let encrypted = encrypt_vectors(&raw_vectors, &key).unwrap();
    let decrypted = decrypt_vectors(&encrypted, &key).unwrap();

    assert_eq!(raw_vectors.len(), decrypted.len());
    for (orig, dec) in raw_vectors.iter().zip(decrypted.iter()) {
        for (a, b) in orig.iter().zip(dec.iter()) {
            assert!((a - b).abs() < 1e-6, "Decrypted values must match original");
        }
    }

    println!("✅ Step 4: Encryption/decryption roundtrip passed");
    println!("   Encrypted size: {} bytes → Shadow Drive", encrypted.len());

    // === STEP 5: Serialization (persistence) ===
    let json = index.to_json().unwrap();
    let restored_index = HNSWIndex::from_json(&json).unwrap();
    let restored_results = restored_index.query(&query, 2).unwrap();

    assert_eq!(restored_results[0].id, results[0].id);
    println!("✅ Step 5: Index serialized and restored — query results match");

    // === FINAL SUMMARY ===
    println!("\n🎉 All pipeline steps passed!\n");
    println!("   Vectors indexed:    {}", index.len());
    println!("   Merkle root:        {} (→ Solana)", &root_hex[..16]);
    println!("   Encrypted payload:  {} bytes (→ Shadow Drive)", encrypted.len());
    println!("   Top query result:   {} ({:.4})", results[0].id, results[0].score);
}
```

---

## EXECUTION INSTRUCTIONS FOR CURSOR

After writing all files above, run these commands in order and fix any errors before moving on:

```bash
# 1. Build everything
cargo build --workspace

# 2. Run all unit tests
cargo test --workspace

# 3. Run integration test
cargo test --test integration_test -- --nocapture

# 4. Run benchmarks (just verify they compile and run)
cargo bench --bench hnsw_bench -- --test
cargo bench --bench distance_bench -- --test

# 5. Run full benchmarks (this takes a few minutes — produces real numbers)
cargo bench --workspace
```

---

## SUCCESS CRITERIA

Phase 1 is complete when ALL of the following are true:

- [ ] `cargo build --workspace` completes with zero errors and zero warnings
- [ ] `cargo test --workspace` shows all tests passing (target: 30+ tests total)
- [ ] `cargo test --test integration_test -- --nocapture` prints the full pipeline summary with all 5 checkmarks
- [ ] `cargo bench` runs and produces real latency numbers in the terminal
- [ ] The HNSW query at 1,000 vectors returns correct nearest neighbor (verifiable by the basic test)
- [ ] Merkle proofs verify correctly for all IDs in a collection
- [ ] Encryption roundtrip test passes with zero data loss
- [ ] Serialization roundtrip test passes (index can be saved and restored)

---

## WHAT NOT TO TOUCH

Do not modify, create, or delete anything in:
- `programs/solvec/` — Phase 2 (Solana Anchor program)
- `sdk/typescript/` — Phase 2
- `sdk/python/` — Phase 2
- `crates/solvec-wasm/` — Phase 2
- `demo/agent-memory/` — Phase 2
- `README.md` — update only if specifically asked
- `THESIS.md` — do not touch

---

## NOTES FOR CURSOR

1. The `U` markers on files in the screenshot means they are Untracked (new files not yet committed to git). Some may have partial content from earlier work. Read each existing file before overwriting — if it already has correct complete content, keep it. If it has stubs or TODOs, replace it with the complete version above.

2. Use `ahash::AHashMap` instead of `std::collections::HashMap` inside the HNSW index internals — it is significantly faster for string keys and is already in the workspace dependencies.

3. The `#[inline(always)]` on the distance `compute` function is intentional and important — this function is called in the innermost loop of HNSW search and must not have call overhead.

4. Do not add any `unwrap()` calls in library code (non-test code). All errors must propagate using `?` and `SolVecError`.

5. All public-facing functions must have a doc comment (`///`).