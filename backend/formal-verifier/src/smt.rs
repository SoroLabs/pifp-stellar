use z3::ast::{Bool, Int};
use z3::Solver;

use crate::error::{FormalVerificationError, Result};
use crate::model::{BoolExpr, IntExpr, PathState, ValueExpr};

#[derive(Debug, Clone)]
pub struct Counterexample {
    pub model: String,
}

pub fn state_has_counterexample(
    state: &PathState,
    property: &BoolExpr,
) -> Result<Option<Counterexample>> {
    let solver = Solver::new();

    let path = lower_bool(&state.path_condition)?;
    solver.assert(&path);

    let property = lower_bool(property)?;
    solver.assert(property.not());

    match solver.check() {
        z3::SatResult::Sat => {
            let model = solver
                .get_model()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "<no model>".to_string());
            Ok(Some(Counterexample { model }))
        }
        z3::SatResult::Unsat => Ok(None),
        z3::SatResult::Unknown => Err(FormalVerificationError::Solver(solver.get_reason_unknown())),
    }
}

pub fn lower_bool(expr: &BoolExpr) -> Result<Bool> {
    Ok(match expr {
        BoolExpr::Const(value) => Bool::from_bool(*value),
        BoolExpr::Var(name) => Bool::new_const(name.as_str()),
        BoolExpr::Not(inner) => lower_bool(inner)?.not(),
        BoolExpr::And(items) => {
            let lowered = items.iter().map(lower_bool).collect::<Result<Vec<_>>>()?;
            let refs = lowered.iter().collect::<Vec<_>>();
            Bool::and(&refs)
        }
        BoolExpr::Or(items) => {
            let lowered = items.iter().map(lower_bool).collect::<Result<Vec<_>>>()?;
            let refs = lowered.iter().collect::<Vec<_>>();
            Bool::or(&refs)
        }
        BoolExpr::Eq(lhs, rhs) => lower_int(lhs)?.eq(lower_int(rhs)?),
        BoolExpr::Ne(lhs, rhs) => lower_int(lhs)?.ne(lower_int(rhs)?),
        BoolExpr::Lt(lhs, rhs) => lower_int(lhs)?.lt(lower_int(rhs)?),
        BoolExpr::Le(lhs, rhs) => lower_int(lhs)?.le(lower_int(rhs)?),
        BoolExpr::Gt(lhs, rhs) => lower_int(lhs)?.gt(lower_int(rhs)?),
        BoolExpr::Ge(lhs, rhs) => lower_int(lhs)?.ge(lower_int(rhs)?),
        BoolExpr::Ite(cond, yes, no) => lower_bool(cond)?.ite(&lower_bool(yes)?, &lower_bool(no)?),
        BoolExpr::Fresh(name) => Bool::new_const(name.as_str()),
    })
}

pub fn lower_int(expr: &IntExpr) -> Result<Int> {
    Ok(match expr {
        IntExpr::Const(value) => Int::from_i64(*value as i64),
        IntExpr::Var(name) => Int::new_const(name.as_str()),
        IntExpr::Fresh(name) => Int::new_const(name.as_str()),
        IntExpr::Add(lhs, rhs) => Int::add(&[&lower_int(lhs)?, &lower_int(rhs)?]),
        IntExpr::Sub(lhs, rhs) => Int::sub(&[&lower_int(lhs)?, &lower_int(rhs)?]),
        IntExpr::Mul(lhs, rhs) => Int::mul(&[&lower_int(lhs)?, &lower_int(rhs)?]),
        IntExpr::Div(lhs, rhs) => lower_int(lhs)?.div(&lower_int(rhs)?),
        IntExpr::Rem(lhs, rhs) => lower_int(lhs)?.rem(&lower_int(rhs)?),
        IntExpr::Ite(cond, yes, no) => lower_bool(cond)?.ite(&lower_int(yes)?, &lower_int(no)?),
    })
}

pub fn bind_value(state: &mut PathState, index: u32, value: ValueExpr) {
    state.locals.insert(index, value);
}
