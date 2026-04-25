use serde::Serialize;

use crate::engine::ArbOpportunity;

#[derive(Debug, Clone)]
pub struct FeePolicy {
    pub base_fee_stroops: u32,
    pub max_fee_stroops: u32,
    pub urgency_multiplier: f64,
    pub profit_capture_bps: u16,
    pub next_ledger_priority: bool,
}

impl Default for FeePolicy {
    fn default() -> Self {
        Self {
            base_fee_stroops: 100,
            max_fee_stroops: 50_000,
            urgency_multiplier: 1.0,
            profit_capture_bps: 1_500,
            next_ledger_priority: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FeeBumpPlan {
    pub base_fee_stroops: u32,
    pub bump_fee_stroops: u32,
    pub total_fee_stroops: u32,
    pub max_fee_stroops: u32,
    pub next_ledger_priority: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    pub notional_stroops: u64,
    pub expected_profit_stroops: i64,
    pub opportunity: ArbOpportunitySummary,
    pub fee_bump: FeeBumpPlan,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbOpportunitySummary {
    pub route: Vec<String>,
    pub gross_multiplier: f64,
    pub gross_profit_bps: f64,
    pub limiting_liquidity: f64,
    pub source_ledger: u64,
}

pub fn build_execution_plan(
    opportunity: &ArbOpportunity,
    notional_stroops: u64,
    policy: &FeePolicy,
) -> Option<ExecutionPlan> {
    let profit_ratio = opportunity.expected_profit_ratio();
    if profit_ratio <= 0.0 || notional_stroops == 0 {
        return None;
    }

    let expected_profit_stroops = (notional_stroops as f64 * profit_ratio).floor() as i64;
    if expected_profit_stroops <= policy.base_fee_stroops as i64 {
        return None;
    }

    let profit_budget = ((expected_profit_stroops as f64) * f64::from(policy.profit_capture_bps)
        / 10_000.0)
        .floor()
        .max(policy.base_fee_stroops as f64) as u32;

    let urgency_scale = if policy.next_ledger_priority {
        1.5
    } else {
        1.0
    };
    let urgency_bonus =
        ((policy.base_fee_stroops as f64) * policy.urgency_multiplier * urgency_scale).ceil()
            as u32;

    let mut total_fee = policy.base_fee_stroops.saturating_add(urgency_bonus);
    total_fee = total_fee.min(policy.max_fee_stroops);
    total_fee = total_fee.min(profit_budget);

    if total_fee <= policy.base_fee_stroops || expected_profit_stroops <= total_fee as i64 {
        return None;
    }

    let opportunity_summary = ArbOpportunitySummary {
        route: opportunity
            .route
            .iter()
            .map(|hop| {
                format!(
                    "{}:{}>{}",
                    hop.pool_id,
                    hop.from.0.as_str(),
                    hop.to.0.as_str()
                )
            })
            .collect(),
        gross_multiplier: opportunity.gross_multiplier,
        gross_profit_bps: opportunity.gross_profit_bps,
        limiting_liquidity: opportunity.limiting_liquidity,
        source_ledger: opportunity.source_ledger,
    };

    Some(ExecutionPlan {
        notional_stroops,
        expected_profit_stroops,
        opportunity: opportunity_summary,
        fee_bump: FeeBumpPlan {
            base_fee_stroops: policy.base_fee_stroops,
            bump_fee_stroops: total_fee - policy.base_fee_stroops,
            total_fee_stroops: total_fee,
            max_fee_stroops: policy.max_fee_stroops,
            next_ledger_priority: policy.next_ledger_priority,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ArbHop, ArbOpportunity};
    use crate::types::AssetId;

    fn opportunity() -> ArbOpportunity {
        ArbOpportunity {
            route: vec![
                ArbHop {
                    pool_id: "pool-a".to_string(),
                    from: AssetId::from("A"),
                    to: AssetId::from("B"),
                    rate: 1.02,
                    fee_bps: 5,
                    updated_ledger: 100,
                    liquidity_cap: 10_000.0,
                },
                ArbHop {
                    pool_id: "pool-b".to_string(),
                    from: AssetId::from("B"),
                    to: AssetId::from("A"),
                    rate: 1.01,
                    fee_bps: 5,
                    updated_ledger: 100,
                    liquidity_cap: 10_000.0,
                },
            ],
            gross_multiplier: 1.0302,
            gross_profit_bps: 302.0,
            limiting_liquidity: 10_000.0,
            source_ledger: 100,
        }
    }

    #[test]
    fn builds_fee_bump_plan() {
        let plan =
            build_execution_plan(&opportunity(), 1_000_000, &FeePolicy::default()).expect("plan");

        assert!(plan.expected_profit_stroops > 0);
        assert!(plan.fee_bump.total_fee_stroops > plan.fee_bump.base_fee_stroops);
    }

    #[test]
    fn rejects_unprofitable_plan() {
        let mut opportunity = opportunity();
        opportunity.gross_multiplier = 1.0;

        assert!(build_execution_plan(&opportunity, 1_000_000, &FeePolicy::default()).is_none());
    }
}
