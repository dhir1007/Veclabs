use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use rand::Rng;
use crate::types::{Vector, QueryResult, DistanceMetric};
use crate::distance;

/// HNSW Index — the performance heart of VecChain
pub struct HNSWIndex {
    // Parameters
    m: usize,              // Max connections per layer
    ef_construction: usize, // Construction beam width
    ef_search: usize,      // Search beam width
    ml: f64,               // Level multiplier

    // Data
    vectors: HashMap<String, Vector>,
    layers: Vec<HashMap<String, Vec<String>>>, // layer -> node -> neighbors
    entry_point: Option<String>,
    max_layer: usize,

    metric: DistanceMetric,
}

impl HNSWIndex {
    pub fn new(m: usize, ef_construction: usize, metric: DistanceMetric) -> Self {
        Self {
            m,
            ef_construction,
            ef_search: ef_construction,
            ml: 1.0 / (m as f64).ln(),
            vectors: HashMap::new(),
            layers: Vec::new(),
            entry_point: None,
            max_layer: 0,
            metric,
        }
    }

    /// Insert a vector into the index
    pub fn insert(&mut self, vector: Vector) -> Result<(), String> {
        let id = vector.id.clone();
        let level = self.random_level();

        // Ensure layers exist
        while self.layers.len() <= level {
            self.layers.push(HashMap::new());
        }

        // Initialize node in each layer up to its level
        for l in 0..=level {
            self.layers[l].insert(id.clone(), Vec::new());
        }

        self.vectors.insert(id.clone(), vector);

        if self.entry_point.is_none() {
            self.entry_point = Some(id.clone());
            self.max_layer = level;
            return Ok(());
        }

        // Connect to existing graph
        self.connect_node(&id, level);

        if level > self.max_layer {
            self.max_layer = level;
            self.entry_point = Some(id);
        }

        Ok(())
    }

    /// Query top-k nearest neighbors
    pub fn query(&self, query_vector: &[f32], top_k: usize) -> Vec<QueryResult> {
        if self.entry_point.is_none() { return vec![]; }

        let entry = self.entry_point.as_ref().unwrap();
        let mut current_nearest = entry.clone();

        // Traverse from top layer down to layer 1
        for layer in (1..=self.max_layer).rev() {
            current_nearest = self.greedy_search_layer(
                query_vector, &current_nearest, layer, 1
            ).into_iter().next().unwrap_or(current_nearest);
        }

        // Search layer 0 with full ef_search
        let candidates = self.greedy_search_layer(
            query_vector, &current_nearest, 0, self.ef_search.max(top_k)
        );

        // Return top_k with scores
        let mut results: Vec<QueryResult> = candidates.into_iter()
            .take(top_k)
            .map(|id| {
                let vec = &self.vectors[&id];
                let score = distance::compute(query_vector, &vec.values, self.metric);
                QueryResult {
                    id,
                    score,
                    metadata: vec.metadata.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut level = 0;
        while rng.gen::<f64>() < self.ml && level < 16 {
            level += 1;
        }
        level
    }

    fn connect_node(&mut self, id: &str, level: usize) {
        // Simplified connection logic — expand this on Day 3
        // Full implementation with pruning goes here
    }

    fn greedy_search_layer(
        &self,
        query: &[f32],
        entry: &str,
        layer: usize,
        ef: usize
    ) -> Vec<String> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut candidates: Vec<(String, f32)> = Vec::new();

        let entry_vec = &self.vectors[entry].values;
        let entry_dist = distance::compute(query, entry_vec, self.metric);

        candidates.push((entry.to_string(), entry_dist));
        visited.insert(entry.to_string());

        let mut result = vec![entry.to_string()];

        // BFS through the graph layer
        let mut i = 0;
        while i < candidates.len() {
            let (current_id, _) = candidates[i].clone();
            i += 1;

            if let Some(neighbors) = self.layers.get(layer).and_then(|l| l.get(&current_id)) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        let n_vec = &self.vectors[neighbor].values;
                        let dist = distance::compute(query, n_vec, self.metric);
                        candidates.push((neighbor.clone(), dist));
                        result.push(neighbor.clone());
                    }
                }
            }
        }

        // Sort by distance and return top ef
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        candidates.into_iter().take(ef).map(|(id, _)| id).collect()
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }
}