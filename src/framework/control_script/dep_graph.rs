use std::collections::{HashMap, VecDeque};

use super::param_mod::ParamValue;
use crate::framework::prelude::*;

/// { "symmetry" -> Param::Hot("t1"), ... }
pub type Node = HashMap<String, ParamValue>;

/// "object_name" -> { "symmetry" -> Param::Hot("t1"), ... }
pub type Graph = HashMap<String, Node>;

pub type EvalOrder = Option<Vec<String>>;

pub struct DepGraph {
    graph: Graph,
    eval_order: EvalOrder,
}

impl DepGraph {
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
            eval_order: None,
        }
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
            self.eval_order = Some(sorted_order)
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
    ) -> (HashMap<String, Vec<String>>, HashMap<String, usize>) {
        // { dependency: [dependents] }
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

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

    pub fn has_dependents(&self, name: &str) -> bool {
        self.eval_order.is_some() && self.graph.contains_key(name)
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

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn clear(&mut self) {
        self.graph.clear();
        self.eval_order = None;
    }
}
