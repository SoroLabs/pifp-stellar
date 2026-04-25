use crate::model::{BoolExpr, IntExpr, PathState, ValueExpr};

#[derive(Debug, Clone)]
pub enum Invariant {
    TotalSupplyBound { supply: String, cap: String },
    PositiveValue { field: String },
    CompletedIsTerminal { completed: String, terminal: String },
    VerifiedImpliesCompleted { verified: String, completed: String },
    NonNegative { field: String },
}

pub trait InvariantProfile {
    fn name(&self) -> &'static str;
    fn invariants(&self) -> Vec<Invariant>;
    fn initial_state(&self) -> PathState;
}

#[derive(Debug, Clone, Default)]
pub struct PifpInvariantProfile;

impl InvariantProfile for PifpInvariantProfile {
    fn name(&self) -> &'static str {
        "pifp"
    }

    fn invariants(&self) -> Vec<Invariant> {
        vec![
            Invariant::TotalSupplyBound {
                supply: "total_supply".to_string(),
                cap: "supply_cap".to_string(),
            },
            Invariant::PositiveValue {
                field: "goal".to_string(),
            },
            Invariant::PositiveValue {
                field: "deadline".to_string(),
            },
            Invariant::CompletedIsTerminal {
                completed: "completed".to_string(),
                terminal: "terminal".to_string(),
            },
            Invariant::VerifiedImpliesCompleted {
                verified: "verified".to_string(),
                completed: "completed".to_string(),
            },
            Invariant::NonNegative {
                field: "donation_count".to_string(),
            },
        ]
    }

    fn initial_state(&self) -> PathState {
        let mut state = PathState::new();
        state
            .locals
            .insert(0, ValueExpr::Int(IntExpr::Var("total_supply".into())));
        state
            .locals
            .insert(1, ValueExpr::Int(IntExpr::Var("supply_cap".into())));
        state
            .locals
            .insert(2, ValueExpr::Bool(BoolExpr::Var("completed".into())));
        state
            .locals
            .insert(3, ValueExpr::Bool(BoolExpr::Var("verified".into())));
        state
            .locals
            .insert(4, ValueExpr::Int(IntExpr::Var("goal".into())));
        state
            .locals
            .insert(5, ValueExpr::Int(IntExpr::Var("deadline".into())));
        state
            .locals
            .insert(6, ValueExpr::Int(IntExpr::Var("donation_count".into())));
        state
    }
}

impl BoolExpr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }
}

impl IntExpr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var(name.into())
    }
}

impl ValueExpr {
    pub fn int(name: impl Into<String>) -> Self {
        Self::Int(IntExpr::Var(name.into()))
    }

    pub fn bool(name: impl Into<String>) -> Self {
        Self::Bool(BoolExpr::Var(name.into()))
    }
}

pub fn invariant_expression(invariant: &Invariant) -> BoolExpr {
    match invariant {
        Invariant::TotalSupplyBound { supply, cap } => BoolExpr::Le(
            Box::new(IntExpr::Var(supply.clone())),
            Box::new(IntExpr::Var(cap.clone())),
        ),
        Invariant::PositiveValue { field } => BoolExpr::Gt(
            Box::new(IntExpr::Var(field.clone())),
            Box::new(IntExpr::Const(0)),
        ),
        Invariant::CompletedIsTerminal {
            completed,
            terminal,
        } => BoolExpr::Or(vec![
            BoolExpr::Not(Box::new(BoolExpr::Var(completed.clone()))),
            BoolExpr::Var(terminal.clone()),
        ]),
        Invariant::VerifiedImpliesCompleted {
            verified,
            completed,
        } => BoolExpr::Or(vec![
            BoolExpr::Not(Box::new(BoolExpr::Var(verified.clone()))),
            BoolExpr::Var(completed.clone()),
        ]),
        Invariant::NonNegative { field } => BoolExpr::Ge(
            Box::new(IntExpr::Var(field.clone())),
            Box::new(IntExpr::Const(0)),
        ),
    }
}
