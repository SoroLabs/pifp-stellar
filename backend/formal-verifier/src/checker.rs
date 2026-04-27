use std::fs;

use tracing::{info, warn};

use crate::error::{FormalVerificationError, Result};
use crate::invariants::{invariant_expression, InvariantProfile};
use crate::model::{BoolExpr, FunctionSummary, Node, Op, PathState, ValueExpr};
use crate::smt::state_has_counterexample;
use crate::wasm::parse_module;

#[derive(Debug, Clone)]
pub struct VerificationConfig {
    pub max_loop_unroll: usize,
    pub focus_exports: Option<Vec<String>>,
    pub fail_closed_on_unsupported: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_loop_unroll: 2,
            focus_exports: None,
            fail_closed_on_unsupported: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationReport {
    pub safe: bool,
    pub checked_functions: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub counterexamples: Vec<CounterexampleReport>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CounterexampleReport {
    pub function: String,
    pub invariant: String,
    pub model: String,
}

pub fn verify_upgrade<P: InvariantProfile>(
    wasm_bytes: &[u8],
    profile: &P,
    config: &VerificationConfig,
) -> Result<VerificationReport> {
    let parsed = parse_module(wasm_bytes)?;
    let mut report = VerificationReport {
        safe: true,
        checked_functions: Vec::new(),
        blocked_reasons: Vec::new(),
        counterexamples: Vec::new(),
    };

    let focus = config.focus_exports.as_ref();
    for function in &parsed.summary.functions {
        if let Some(names) = focus {
            if let Some(name) = &function.export_name {
                if !names.iter().any(|wanted| wanted == name) {
                    continue;
                }
            } else {
                continue;
            }
        }

        report.checked_functions.push(function_label(function));

        if config.fail_closed_on_unsupported && !function.unsupported_ops.is_empty() {
            report.safe = false;
            report.blocked_reasons.push(format!(
                "{} contains unsupported opcodes: {}",
                function_label(function),
                function.unsupported_ops.join(", ")
            ));
            continue;
        }

        let function_report = verify_function(function, profile, config)?;
        if !function_report.safe {
            report.safe = false;
        }
        report
            .counterexamples
            .extend(function_report.counterexamples);
        report
            .blocked_reasons
            .extend(function_report.blocked_reasons);
    }

    if !report.safe {
        return Err(FormalVerificationError::Blocked(
            serde_json::to_string_pretty(&report)?,
        ));
    }

    Ok(report)
}

fn verify_function<P: InvariantProfile>(
    function: &FunctionSummary,
    profile: &P,
    config: &VerificationConfig,
) -> Result<VerificationReport> {
    let mut report = VerificationReport {
        safe: true,
        checked_functions: vec![function_label(function)],
        blocked_reasons: Vec::new(),
        counterexamples: Vec::new(),
    };

    let initial = profile.initial_state();
    let leaves = execute_program(&function.program.nodes, initial, config.max_loop_unroll)?;

    for leaf in leaves {
        for invariant in profile.invariants() {
            let property = invariant_expression(&invariant);
            match state_has_counterexample(&leaf, &property)? {
                Some(model) => {
                    report.safe = false;
                    report.counterexamples.push(CounterexampleReport {
                        function: function_label(function),
                        invariant: format!("{invariant:?}"),
                        model: model.model,
                    });
                }
                None => {}
            }
        }
    }

    if !report.safe {
        warn!(
            function = %function_label(function),
            "counterexample found during formal verification"
        );
    } else {
        info!(function = %function_label(function), "function proved safe");
    }

    Ok(report)
}

fn execute_program(
    nodes: &[Node],
    initial: PathState,
    max_loop_unroll: usize,
) -> Result<Vec<PathState>> {
    let mut states = vec![initial];
    for node in nodes {
        let mut next = Vec::new();
        for state in states {
            next.extend(execute_node(node, state, max_loop_unroll)?);
        }
        states = next;
    }
    Ok(states)
}

fn execute_node(node: &Node, state: PathState, max_loop_unroll: usize) -> Result<Vec<PathState>> {
    match node {
        Node::Op(op) => execute_op(op, state),
        Node::Block(nodes) => execute_block(nodes, state, max_loop_unroll),
        Node::Loop(nodes) => execute_loop(nodes, state, max_loop_unroll),
        Node::If {
            then_nodes,
            else_nodes,
        } => execute_if(then_nodes, else_nodes, state, max_loop_unroll),
    }
}

fn execute_block(
    nodes: &[Node],
    state: PathState,
    max_loop_unroll: usize,
) -> Result<Vec<PathState>> {
    execute_program(nodes, state, max_loop_unroll)
}

fn execute_loop(
    nodes: &[Node],
    state: PathState,
    max_loop_unroll: usize,
) -> Result<Vec<PathState>> {
    let mut frontier = vec![state];
    let mut leaves = Vec::new();

    for _ in 0..max_loop_unroll.max(1) {
        let mut next = Vec::new();
        for state in frontier {
            let iterated = execute_program(nodes, state.clone(), max_loop_unroll)?;
            if iterated.is_empty() {
                leaves.push(state);
            } else {
                next.extend(iterated);
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }

    leaves.extend(frontier);
    Ok(leaves)
}

fn execute_if(
    then_nodes: &[Node],
    else_nodes: &[Node],
    mut state: PathState,
    max_loop_unroll: usize,
) -> Result<Vec<PathState>> {
    let cond = state
        .pop()
        .ok_or_else(|| FormalVerificationError::Symbolic("if without condition".to_string()))?;
    let cond = PathState::require_bool(cond).map_err(FormalVerificationError::Symbolic)?;

    let mut true_state = state.clone();
    true_state.path_condition = BoolExpr::And(vec![true_state.path_condition, cond.clone()]);
    let mut false_state = state;
    false_state.path_condition = BoolExpr::And(vec![
        false_state.path_condition,
        BoolExpr::Not(Box::new(cond)),
    ]);

    let mut leaves = execute_program(then_nodes, true_state, max_loop_unroll)?;
    leaves.extend(execute_program(else_nodes, false_state, max_loop_unroll)?);
    Ok(leaves)
}

fn execute_op(op: &Op, mut state: PathState) -> Result<Vec<PathState>> {
    match op {
        Op::Nop => Ok(vec![state]),
        Op::Unreachable => Ok(Vec::new()),
        Op::Drop => {
            let _ = state.pop();
            Ok(vec![state])
        }
        Op::Select => {
            let cond = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("select missing condition".to_string())
            })?;
            let yes = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("select missing then value".to_string())
            })?;
            let no = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("select missing else value".to_string())
            })?;

            let cond = PathState::require_bool(cond).map_err(FormalVerificationError::Symbolic)?;
            let yes = PathState::require_int(yes).map_err(FormalVerificationError::Symbolic)?;
            let no = PathState::require_int(no).map_err(FormalVerificationError::Symbolic)?;

            state.push(ValueExpr::Int(crate::model::IntExpr::Ite(
                Box::new(cond),
                Box::new(yes),
                Box::new(no),
            )));
            Ok(vec![state])
        }
        Op::Return => Ok(Vec::new()),
        Op::LocalGet(index) => {
            let value = state
                .locals
                .entry(*index)
                .or_insert_with(|| {
                    ValueExpr::Int(crate::model::IntExpr::Fresh(format!("local_{index}")))
                })
                .clone();
            state.push(value);
            Ok(vec![state])
        }
        Op::LocalSet(index) => {
            let value = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("local.set missing value".to_string())
            })?;
            state.locals.insert(*index, value);
            Ok(vec![state])
        }
        Op::LocalTee(index) => {
            let value = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("local.tee missing value".to_string())
            })?;
            state.locals.insert(*index, value.clone());
            state.push(value);
            Ok(vec![state])
        }
        Op::GlobalGet(index) => {
            let value = state
                .globals
                .entry(*index)
                .or_insert_with(|| {
                    ValueExpr::Int(crate::model::IntExpr::Fresh(format!("global_{index}")))
                })
                .clone();
            state.push(value);
            Ok(vec![state])
        }
        Op::GlobalSet(index) => {
            let value = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("global.set missing value".to_string())
            })?;
            state.globals.insert(*index, value);
            Ok(vec![state])
        }
        Op::I32Const(value) => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Const(*value as i128)));
            Ok(vec![state])
        }
        Op::I64Const(value) => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Const(*value as i128)));
            Ok(vec![state])
        }
        Op::I32Eqz | Op::I64Eqz => {
            let value = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("eqz missing value".to_string())
            })?;
            let value = PathState::require_int(value).map_err(FormalVerificationError::Symbolic)?;
            state.push(ValueExpr::Bool(crate::model::BoolExpr::Eq(
                Box::new(value),
                Box::new(crate::model::IntExpr::Const(0)),
            )));
            Ok(vec![state])
        }
        Op::I32Add | Op::I64Add => binary_int(state, crate::model::IntExpr::Add),
        Op::I32Sub | Op::I64Sub => binary_int(state, crate::model::IntExpr::Sub),
        Op::I32Mul | Op::I64Mul => binary_int(state, crate::model::IntExpr::Mul),
        Op::I32DivS | Op::I32DivU | Op::I64DivS | Op::I64DivU => {
            binary_int(state, crate::model::IntExpr::Div)
        }
        Op::I32RemS | Op::I32RemU | Op::I64RemS | Op::I64RemU => {
            binary_int(state, crate::model::IntExpr::Rem)
        }
        Op::I32And | Op::I64And | Op::I32Or | Op::I64Or | Op::I32Xor | Op::I64Xor => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Fresh(format!(
                "bitop_{op:?}"
            ))));
            Ok(vec![state])
        }
        Op::I32Shl | Op::I64Shl | Op::I32ShrS | Op::I64ShrS | Op::I32ShrU | Op::I64ShrU => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Fresh(format!(
                "shift_{op:?}"
            ))));
            Ok(vec![state])
        }
        Op::EqI32 | Op::EqI64 => binary_bool(state, crate::model::BoolExpr::Eq),
        Op::NeI32 | Op::NeI64 => binary_bool(state, crate::model::BoolExpr::Ne),
        Op::LtSI32 | Op::LtSI64 => binary_bool(state, crate::model::BoolExpr::Lt),
        Op::LeSI32 | Op::LeSI64 => binary_bool(state, crate::model::BoolExpr::Le),
        Op::GtSI32 | Op::GtSI64 => binary_bool(state, crate::model::BoolExpr::Gt),
        Op::GeSI32 | Op::GeSI64 => binary_bool(state, crate::model::BoolExpr::Ge),
        Op::Br(depth) => Ok(vec![br_state(state, *depth)]),
        Op::BrIf(depth) => {
            let cond = state.pop().ok_or_else(|| {
                FormalVerificationError::Symbolic("br_if missing condition".to_string())
            })?;
            let cond = PathState::require_bool(cond).map_err(FormalVerificationError::Symbolic)?;
            let mut taken = state.clone();
            taken.path_condition = BoolExpr::And(vec![taken.path_condition, cond.clone()]);
            let mut not_taken = state;
            not_taken.path_condition = BoolExpr::And(vec![
                not_taken.path_condition,
                BoolExpr::Not(Box::new(cond)),
            ]);
            let mut out = vec![br_state(taken, *depth)];
            out.push(not_taken);
            Ok(out)
        }
        Op::Call(function_index) => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Fresh(format!(
                "call_{function_index}"
            ))));
            Ok(vec![state])
        }
        Op::CallIndirect => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Fresh(
                "call_indirect".into(),
            )));
            Ok(vec![state])
        }
        Op::LoadFreshInt(tag) => {
            state.push(ValueExpr::Int(crate::model::IntExpr::Fresh(tag.clone())));
            Ok(vec![state])
        }
        Op::Block | Op::Loop | Op::If | Op::Else | Op::End => Ok(vec![state]),
        Op::I32And
        | Op::I32Or
        | Op::I32Xor
        | Op::I32Shl
        | Op::I32ShrS
        | Op::I32ShrU
        | Op::I64And
        | Op::I64Or
        | Op::I64Xor
        | Op::I64Shl
        | Op::I64ShrS
        | Op::I64ShrU => unreachable!(),
    }
}

fn binary_int<F>(mut state: PathState, make: F) -> Result<Vec<PathState>>
where
    F: Fn(Box<crate::model::IntExpr>, Box<crate::model::IntExpr>) -> crate::model::IntExpr,
{
    let rhs = state
        .pop()
        .ok_or_else(|| FormalVerificationError::Symbolic("missing rhs".to_string()))?;
    let lhs = state
        .pop()
        .ok_or_else(|| FormalVerificationError::Symbolic("missing lhs".to_string()))?;
    let rhs = PathState::require_int(rhs).map_err(FormalVerificationError::Symbolic)?;
    let lhs = PathState::require_int(lhs).map_err(FormalVerificationError::Symbolic)?;
    state.push(ValueExpr::Int(make(Box::new(lhs), Box::new(rhs))));
    Ok(vec![state])
}

fn binary_bool<F>(mut state: PathState, make: F) -> Result<Vec<PathState>>
where
    F: Fn(Box<crate::model::IntExpr>, Box<crate::model::IntExpr>) -> crate::model::BoolExpr,
{
    let rhs = state
        .pop()
        .ok_or_else(|| FormalVerificationError::Symbolic("missing rhs".to_string()))?;
    let lhs = state
        .pop()
        .ok_or_else(|| FormalVerificationError::Symbolic("missing lhs".to_string()))?;
    let rhs = PathState::require_int(rhs).map_err(FormalVerificationError::Symbolic)?;
    let lhs = PathState::require_int(lhs).map_err(FormalVerificationError::Symbolic)?;
    state.push(ValueExpr::Bool(make(Box::new(lhs), Box::new(rhs))));
    Ok(vec![state])
}

fn br_state(mut state: PathState, depth: u32) -> PathState {
    state.notes.push(format!("branch({depth})"));
    state
}

fn function_label(function: &FunctionSummary) -> String {
    function
        .export_name
        .clone()
        .unwrap_or_else(|| format!("fn_{}", function.index))
}
