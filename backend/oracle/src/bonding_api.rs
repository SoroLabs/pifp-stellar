use axum::{
    extract::Query,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use crate::bonding_curve::BondingCurve;

#[derive(Deserialize)]
pub struct SimulationParams {
    pub amount: f64,
    pub is_buy: bool,
    pub reserve: Option<f64>,
    pub supply: Option<f64>,
    pub ratio: Option<f64>,
}

pub async fn simulate_trade(
    Query(params): Query<SimulationParams>,
) -> Json<crate::bonding_curve::TradeImpact> {
    let curve = BondingCurve::new(
        params.reserve.unwrap_or(1000000.0),
        params.supply.unwrap_or(10000000.0),
        params.ratio.unwrap_or(0.5),
    );

    let impact = curve.simulate_trade(params.amount, params.is_buy);
    Json(impact)
}

pub fn router() -> Router {
    Router::new()
        .route("/simulate", get(simulate_trade))
}
