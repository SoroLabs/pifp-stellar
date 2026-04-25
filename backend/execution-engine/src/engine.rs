use std::collections::{HashMap, HashSet};

use crate::types::{AssetId, EdgeView, PoolSnapshot};

const EPSILON: f64 = 1e-12;

#[derive(Debug, Clone)]
pub struct ArbHop {
    pub pool_id: String,
    pub from: AssetId,
    pub to: AssetId,
    pub rate: f64,
    pub fee_bps: u32,
    pub updated_ledger: u64,
    pub liquidity_cap: f64,
}

#[derive(Debug, Clone)]
pub struct ArbOpportunity {
    pub route: Vec<ArbHop>,
    pub gross_multiplier: f64,
    pub gross_profit_bps: f64,
    pub limiting_liquidity: f64,
    pub source_ledger: u64,
}

impl ArbOpportunity {
    pub fn expected_profit_ratio(&self) -> f64 {
        self.gross_multiplier - 1.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArbSearchConfig {
    pub min_profit_bps: f64,
}

impl Default for ArbSearchConfig {
    fn default() -> Self {
        Self {
            min_profit_bps: 2.5,
        }
    }
}

#[derive(Debug, Default)]
pub struct BellmanFordFinder {
    pub config: ArbSearchConfig,
}

impl BellmanFordFinder {
    pub fn new(config: ArbSearchConfig) -> Self {
        Self { config }
    }

    pub fn find_best_opportunity(&self, snapshots: &[PoolSnapshot]) -> Option<ArbOpportunity> {
        let edges = build_edges(snapshots);
        if edges.is_empty() {
            return None;
        }

        let nodes = unique_nodes(&edges);
        if nodes.len() < 2 {
            return None;
        }

        let index = node_index(&nodes);
        let mut dist = vec![0.0f64; nodes.len()];
        let mut pred_vertex: Vec<Option<usize>> = vec![None; nodes.len()];
        let mut pred_edge: Vec<Option<usize>> = vec![None; nodes.len()];

        for _ in 0..nodes.len().saturating_sub(1) {
            let mut changed = false;
            for (edge_idx, edge) in edges.iter().enumerate() {
                let from = index[&edge.from];
                let to = index[&edge.to];
                let candidate = dist[from] + edge.weight();
                if candidate + EPSILON < dist[to] {
                    dist[to] = candidate;
                    pred_vertex[to] = Some(from);
                    pred_edge[to] = Some(edge_idx);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let mut best: Option<ArbOpportunity> = None;
        let mut seen_cycles = HashSet::new();

        for edge in &edges {
            let from = index[&edge.from];
            let to = index[&edge.to];
            if dist[from] + edge.weight() + EPSILON >= dist[to] {
                continue;
            }

            let Some(cycle_vertices) = reconstruct_cycle(to, &pred_vertex, nodes.len()) else {
                continue;
            };
            let Some(cycle_edge_ids) = reconstruct_cycle_edges(&cycle_vertices, &pred_edge) else {
                continue;
            };

            let route = build_route(&cycle_edge_ids, &edges);
            if route.len() < 2 {
                continue;
            }

            let canonical = canonical_route_key(&route);
            if !seen_cycles.insert(canonical) {
                continue;
            }

            let opportunity = opportunity_from_route(route);
            if opportunity.gross_profit_bps < self.config.min_profit_bps {
                continue;
            }

            match &best {
                Some(current) if current.gross_multiplier >= opportunity.gross_multiplier => {}
                _ => best = Some(opportunity),
            }
        }

        best
    }
}

fn build_edges(snapshots: &[PoolSnapshot]) -> Vec<WeightedEdge> {
    let mut edges = Vec::new();
    for snapshot in snapshots {
        if let Some([forward, reverse]) = snapshot.directed_edges() {
            edges.push(WeightedEdge::from_view(forward));
            edges.push(WeightedEdge::from_view(reverse));
        }
    }
    edges
}

fn unique_nodes(edges: &[WeightedEdge]) -> Vec<AssetId> {
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();
    for edge in edges {
        if seen.insert(edge.from.clone()) {
            nodes.push(edge.from.clone());
        }
        if seen.insert(edge.to.clone()) {
            nodes.push(edge.to.clone());
        }
    }
    nodes
}

fn node_index(nodes: &[AssetId]) -> HashMap<AssetId, usize> {
    nodes
        .iter()
        .cloned()
        .enumerate()
        .map(|(idx, node)| (node, idx))
        .collect()
}

fn reconstruct_cycle(
    mut vertex: usize,
    pred_vertex: &[Option<usize>],
    node_count: usize,
) -> Option<Vec<usize>> {
    for _ in 0..node_count {
        vertex = pred_vertex.get(vertex).and_then(|v| *v)?;
    }

    let start = vertex;
    let mut cycle = Vec::new();
    let mut seen = HashSet::new();
    let mut current = start;

    loop {
        if !seen.insert(current) {
            break;
        }
        cycle.push(current);
        current = pred_vertex.get(current).and_then(|v| *v)?;
        if current == start {
            break;
        }
    }

    cycle.push(start);
    cycle.reverse();
    Some(cycle)
}

fn reconstruct_cycle_edges(
    cycle_vertices: &[usize],
    pred_edge: &[Option<usize>],
) -> Option<Vec<usize>> {
    if cycle_vertices.len() < 2 {
        return None;
    }

    let mut edges = Vec::new();
    for window in cycle_vertices.windows(2) {
        let to = window[1];
        let edge_idx = pred_edge.get(to).and_then(|v| *v)?;
        edges.push(edge_idx);
    }
    Some(edges)
}

fn build_route(edge_ids: &[usize], edges: &[WeightedEdge]) -> Vec<ArbHop> {
    edge_ids
        .iter()
        .filter_map(|edge_id| edges.get(*edge_id))
        .map(|edge| ArbHop {
            pool_id: edge.pool_id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            rate: edge.rate,
            fee_bps: edge.fee_bps,
            updated_ledger: edge.updated_ledger,
            liquidity_cap: edge.liquidity_cap,
        })
        .collect()
}

fn opportunity_from_route(route: Vec<ArbHop>) -> ArbOpportunity {
    let gross_multiplier = route.iter().fold(1.0, |acc, hop| acc * hop.rate);
    let limiting_liquidity = route
        .iter()
        .map(|hop| hop.liquidity_cap)
        .fold(f64::INFINITY, f64::min);
    let source_ledger = route
        .iter()
        .map(|hop| hop.updated_ledger)
        .min()
        .unwrap_or(0);

    ArbOpportunity {
        gross_profit_bps: (gross_multiplier - 1.0) * 10_000.0,
        route,
        gross_multiplier,
        limiting_liquidity,
        source_ledger,
    }
}

fn canonical_route_key(route: &[ArbHop]) -> String {
    route
        .iter()
        .map(|hop| {
            format!(
                "{}:{}>{}",
                hop.pool_id,
                hop.from.0.as_str(),
                hop.to.0.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[derive(Debug, Clone)]
struct WeightedEdge {
    from: AssetId,
    to: AssetId,
    rate: f64,
    weight: f64,
    pool_id: String,
    fee_bps: u32,
    updated_ledger: u64,
    liquidity_cap: f64,
}

impl WeightedEdge {
    fn from_view(view: EdgeView) -> Self {
        Self {
            weight: -view.rate.ln(),
            from: view.from,
            to: view.to,
            rate: view.rate,
            pool_id: view.pool_id,
            fee_bps: view.fee_bps,
            updated_ledger: view.updated_ledger,
            liquidity_cap: view.liquidity_cap,
        }
    }

    fn weight(&self) -> f64 {
        self.weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AssetId;

    fn snapshot(
        pool_id: &str,
        base: &str,
        quote: &str,
        base_reserve: f64,
        quote_reserve: f64,
    ) -> PoolSnapshot {
        PoolSnapshot {
            pool_id: pool_id.to_string(),
            base_asset: AssetId::from(base),
            quote_asset: AssetId::from(quote),
            base_reserve,
            quote_reserve,
            fee_bps: 5,
            updated_ledger: 100,
        }
    }

    #[test]
    fn finds_profitable_cycle() {
        let snapshots = vec![
            snapshot("pool-ab", "A", "B", 1_000.0, 1_020.0),
            snapshot("pool-bc", "B", "C", 1_000.0, 1_020.0),
            snapshot("pool-ca", "C", "A", 1_000.0, 1_020.0),
        ];

        let finder = BellmanFordFinder::new(ArbSearchConfig {
            min_profit_bps: 1.0,
        });

        let opportunity = finder.find_best_opportunity(&snapshots).expect("cycle");

        assert!(opportunity.gross_multiplier > 1.0);
        assert!(opportunity.gross_profit_bps > 1.0);
        assert_eq!(opportunity.route.len(), 3);
    }

    #[test]
    fn ignores_unprofitable_graphs() {
        let snapshots = vec![
            snapshot("pool-ab", "A", "B", 1_000.0, 999.0),
            snapshot("pool-bc", "B", "C", 1_000.0, 999.0),
            snapshot("pool-ca", "C", "A", 1_000.0, 999.0),
        ];

        let finder = BellmanFordFinder::new(ArbSearchConfig {
            min_profit_bps: 5.0,
        });

        assert!(finder.find_best_opportunity(&snapshots).is_none());
    }
}
