use std::collections::VecDeque;

use nannou_egui::egui::ahash::HashSet;

use super::param_mod::ParamValue;
use crate::framework::prelude::*;

pub type Node = HashMap<String, ParamValue>;
pub type Graph = HashMap<String, Node>;
pub type EvalOrder = Option<Vec<String>>;

/// A directed graph structure that manages parameter dependency relationships.
///
/// The `DepGraph` keeps track of which control nodes ("consumers") depend on
/// other nodes ("prerequisites") and calculates the correct order in which they
/// should be evaluated..
///
/// # Usage Flow
///
/// 1. Create a new `DepGraph` instance
/// 2. Add nodes with [`DepGraph::insert_node`], where each node represents a
///    control with prerequisites
/// 3. Call [`DepGraph::build_graph`] to analyze all prerequisites and compute
///    the evaluation order
/// 4. Use [`DepGraph::order`] to get the proper evaluation sequence and
///    [`DepGraph::is_prerequisite`] to check if a node is required for other
///    calculations
/// ```
#[derive(Debug, Default)]
pub struct DepGraph {
    /// Stores original node definitions with their parameters and dependencies
    ///
    /// # Example
    /// ```
    /// { "symmetry" -> Param::Hot("t1"), ... }
    /// ```
    node_defs: Graph,

    /// Computed evaluation order for prerequisite nodes
    eval_order: EvalOrder,

    /// Lookup map for faster dependency checking
    prerequisites: HashMap<String, bool>,
}

impl DepGraph {
    pub fn is_prerequisite(&self, name: &str) -> bool {
        *self.prerequisites.get(name).unwrap_or(&false)
    }

    pub fn order(&self) -> &EvalOrder {
        &self.eval_order
    }

    pub fn node(&self, name: &str) -> Option<&Node> {
        self.node_defs.get(name)
    }

    pub fn insert_node(&mut self, name: &str, node: Node) {
        self.node_defs.insert(name.to_string(), node);
    }

    pub fn clear(&mut self) {
        self.node_defs.clear();
        self.eval_order = None;
    }

    /// Builds the prerequisite evaluation order using a modified Kahn's
    /// Algorithm for topological sorting
    pub fn build_graph(&mut self) {
        let (graph, mut in_degree) = self.extract_relationships();

        let mut actual_deps: HashSet<String> = HashSet::default();
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut sorted_order: Vec<String> = Vec::new();

        // Ensure we don't incorrectly add the consumer in addition to its
        // prerequisites (if the consumer itself is not a prerequisite)
        for params in self.node_defs.values() {
            for value in params.values() {
                if let ParamValue::Hot(hot_name) = value {
                    actual_deps.insert(hot_name.clone());
                }
            }
        }

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            if actual_deps.contains(&node) {
                sorted_order.push(node.clone());
            }

            if let Some(deps) = graph.get(&node) {
                for dep in deps {
                    if let Some(count) = in_degree.get_mut(dep) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        if sorted_order.len() == actual_deps.len() {
            for dep in sorted_order.iter() {
                self.prerequisites.insert(dep.to_string(), true);
            }
            self.eval_order =
                ternary!(sorted_order.is_empty(), None, Some(sorted_order));
        } else {
            self.eval_order = None;
            warn!(
                "cycle detected. sorted_order: {:?}, in_degree: {:?}",
                sorted_order, in_degree
            );
        }
    }

    /// Analyzes the node definitions to identify prerequisite relationships.
    ///
    /// Returns:
    /// - A map of each prerequisite to the nodes that consume it
    /// - A map tracking the number of prerequisites each node depends on
    fn extract_relationships(
        &self,
    ) -> (HashMap<String, Vec<String>>, HashMap<String, usize>) {
        // graph = { "prerequisite_node": ["consumer_node"] }
        let mut graph: HashMap<String, Vec<String>> = HashMap::default();

        // Number of prerequisite nodes each consumer depends on
        let mut in_degree: HashMap<String, usize> = HashMap::default();

        for (node_name, params) in self.node_defs.iter() {
            // value = Hot("prerequisite_node")
            for value in params.values() {
                // hot_name = "prerequisite_node"
                if let ParamValue::Hot(hot_name) = value {
                    in_degree.entry(hot_name.clone()).or_insert(0);

                    graph
                        .entry(hot_name.clone())
                        .or_default()
                        .push(node_name.clone());

                    *in_degree.entry(node_name.clone()).or_insert(0) += 1;
                }
            }
        }

        (graph, in_degree)
    }
}
