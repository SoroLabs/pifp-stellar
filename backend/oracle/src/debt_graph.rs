use std::collections::{HashMap, VecDeque};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebtGraph {
    pub edges: Vec<Edge>,
}

impl DebtGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_edge(&mut self, from: &str, to: &str, amount: u64) {
        self.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
            amount,
        });
    }

    /// Simplifies the graph by canceling out cycles.
    pub fn minimize_debt(&mut self) {
        loop {
            if let Some(cycle) = self.find_cycle() {
                // Find minimum amount in the cycle
                let min_amount = cycle.iter().map(|e| e.amount).min().unwrap_or(0);
                if min_amount == 0 { break; }

                // Subtract min_amount from all edges in the cycle
                for cycle_edge in cycle {
                    if let Some(edge) = self.edges.iter_mut().find(|e| 
                        e.from == cycle_edge.from && e.to == cycle_edge.to && e.amount >= min_amount
                    ) {
                        edge.amount -= min_amount;
                    }
                }
                
                // Remove zero-amount edges
                self.edges.retain(|e| e.amount > 0);
            } else {
                break;
            }
        }
    }

    fn find_cycle(&self) -> Option<Vec<Edge>> {
        let mut adj = HashMap::new();
        for edge in &self.edges {
            adj.entry(edge.from.clone()).or_insert_with(Vec::new).push(edge);
        }

        let nodes: Vec<_> = adj.keys().cloned().collect();
        let mut visited = HashMap::new(); // 0: unvisited, 1: visiting, 2: visited
        let mut path = Vec::new();

        for node in nodes {
            if visited.get(&node).unwrap_or(&0) == &0 {
                if let Some(cycle) = self.dfs_find_cycle(&node, &adj, &mut visited, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_find_cycle(
        &self,
        node: &String,
        adj: &HashMap<String, Vec<&Edge>>,
        visited: &mut HashMap<String, i8>,
        path: &mut Vec<Edge>,
    ) -> Option<Vec<Edge>> {
        visited.insert(node.clone(), 1);

        if let Some(edges) = adj.get(node) {
            for edge in edges {
                path.push((*edge).clone());
                if visited.get(&edge.to).unwrap_or(&0) == &1 {
                    // Cycle detected! Extract the cycle from path
                    let start_idx = path.iter().position(|e| e.from == edge.to).unwrap();
                    return Some(path[start_idx..].to_vec());
                } else if visited.get(&edge.to).unwrap_or(&0) == &0 {
                    if let Some(cycle) = self.dfs_find_cycle(&edge.to, adj, visited, path) {
                        return Some(cycle);
                    }
                }
                path.pop();
            }
        }

        visited.insert(node.clone(), 2);
        None
    }

    pub fn get_settlements(&self) -> Vec<Edge> {
        self.edges.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_cancellation() {
        let mut graph = DebtGraph::new();
        graph.add_edge("A", "B", 100);
        graph.add_edge("B", "C", 100);
        graph.add_edge("C", "A", 100);

        graph.minimize_debt();
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn test_partial_cycle_cancellation() {
        let mut graph = DebtGraph::new();
        graph.add_edge("A", "B", 100);
        graph.add_edge("B", "C", 100);
        graph.add_edge("C", "A", 50);

        graph.minimize_debt();
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges.iter().any(|e| e.from == "A" && e.to == "B" && e.amount == 50));
        assert!(graph.edges.iter().any(|e| e.from == "B" && e.to == "C" && e.amount == 50));
    }
}
