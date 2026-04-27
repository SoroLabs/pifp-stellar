use axum::{
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use crate::debt_graph::{DebtGraph, Edge};

#[derive(Deserialize)]
pub struct OptimizeDebtRequest {
    pub edges: Vec<Edge>,
}

#[derive(Serialize)]
pub struct OptimizeDebtResponse {
    pub original: Vec<Edge>,
    pub optimized: Vec<Edge>,
    pub settlements: Vec<Edge>,
}

pub fn router() -> Router<()> {
    Router::new().route("/optimize", post(handle_optimize_debt))
}

async fn handle_optimize_debt(
    Json(payload): Json<OptimizeDebtRequest>,
) -> impl IntoResponse {
    let mut graph = DebtGraph::new();
    for edge in &payload.edges {
        graph.add_edge(&edge.from, &edge.to, edge.amount);
    }

    let original = graph.edges.clone();
    graph.minimize_debt();
    let optimized = graph.edges.clone();
    let settlements = graph.get_settlements();

    Json(OptimizeDebtResponse {
        original,
        optimized,
        settlements,
    })
}
