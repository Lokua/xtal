use rustc_hash::FxHashMap;
use std::collections::VecDeque;

use super::param_mod::ParamValue;
use crate::framework::prelude::*;

/// { "symmetry" -> Param::Hot("t1"), ... }
pub type Node = FxHashMap<String, ParamValue>;

/// "t2" -> { "symmetry" -> Param::Hot("t1"), ... }
pub type Graph = FxHashMap<String, Node>;

pub type EvalOrder = Option<Vec<String>>;

#[derive(Debug)]
pub struct DepGraph {
    graph: Graph,
    eval_order: EvalOrder,
    /// Provides faster lookups than the eval_order list
    is_dep: FxHashMap<String, bool>,
}

impl DepGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::default(),
            eval_order: None,
            is_dep: FxHashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn has_dependents(&self, name: &str) -> bool {
        self.eval_order.is_some() && self.graph.contains_key(name)
    }

    pub fn is_dependency(&self, name: &str) -> bool {
        *self.is_dep.get(name).unwrap_or(&false)
    }

    pub fn node(&self, name: &str) -> Option<&Node> {
        self.graph.get(name)
    }

    pub fn insert_node(&mut self, name: &str, node: Node) {
        self.graph.insert(name.to_string(), node);
    }

    pub fn order(&self) -> &EvalOrder {
        &self.eval_order
    }

    #[allow(unused)]
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn clear(&mut self) {
        self.graph.clear();
        self.eval_order = None;
    }

    /// Builds the final eval_order list using Kahn’s Algorithm (topological
    /// sort).
    pub fn build_graph(&mut self) {
        let (graph, mut in_degree) = self.create_reverse_dep_graph_and_order();

        let mut queue: VecDeque<String> = VecDeque::new();
        let mut sorted_order: Vec<String> = Vec::new();

        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            sorted_order.push(node.clone());

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

        if sorted_order.len() == in_degree.len() {
            for dep in sorted_order.iter() {
                self.is_dep.insert(dep.to_string(), true);
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

    fn create_reverse_dep_graph_and_order(
        &self,
    ) -> (FxHashMap<String, Vec<String>>, FxHashMap<String, usize>) {
        // { dependency: [dependents] }
        let mut graph: FxHashMap<String, Vec<String>> = FxHashMap::default();
        let mut in_degree: FxHashMap<String, usize> = FxHashMap::default();

        // self.graph = { "hot_effect": { param: Hot("hot_anim") }, ... }
        // node_name = "hot_effect"
        // node = { param: Hot("hot_anim") }
        for (node_name, params) in self.graph.iter() {
            // value = Hot("hot_anim")
            for (_, value) in params.iter() {
                // hot_value = "hot_anim"
                if let ParamValue::Hot(hot_value) = value {
                    in_degree.entry(hot_value.clone()).or_insert(0);

                    // graph = { "hot_anim": ["hot_effect"] }
                    // "hot_effect depends on hot_anim"
                    graph
                        .entry(hot_value.clone())
                        .or_default()
                        .push(node_name.clone());

                    *in_degree.entry(node_name.clone()).or_insert(0) += 1;
                }
            }
        }

        (graph, in_degree)
    }
}
