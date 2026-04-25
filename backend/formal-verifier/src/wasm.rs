use std::collections::BTreeMap;

use wasmparser::{Encoding, Operator, Parser, Payload};

use crate::error::{FormalVerificationError, Result};
use crate::model::{FunctionSummary, ModuleSummary, Node, Op, Program, ValueExpr};

#[derive(Debug)]
struct Frame {
    kind: FrameKind,
    then_nodes: Vec<Node>,
    else_nodes: Vec<Node>,
    in_else: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Root,
    Block,
    Loop,
    If,
}

impl Frame {
    fn push(&mut self, node: Node) {
        if self.in_else {
            self.else_nodes.push(node);
        } else {
            self.then_nodes.push(node);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub summary: ModuleSummary,
    pub exported_functions: BTreeMap<String, u32>,
}

pub fn parse_module(bytes: &[u8]) -> Result<ParsedModule> {
    let mut exported_functions = BTreeMap::new();
    let mut functions = Vec::new();
    let mut current_function_index: u32 = 0;
    let mut export_names: BTreeMap<u32, String> = BTreeMap::new();

    let parser = Parser::new(0);
    for payload in parser.parse_all(bytes) {
        let payload = payload.map_err(|e| FormalVerificationError::WasmParse(e.to_string()))?;
        match payload {
            Payload::Version { encoding, .. } => {
                if encoding != Encoding::Module {
                    return Err(FormalVerificationError::WasmParse(
                        "only core wasm modules are supported".to_string(),
                    ));
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export =
                        export.map_err(|e| FormalVerificationError::WasmParse(e.to_string()))?;
                    if let wasmparser::ExternalKind::Func = export.kind {
                        exported_functions.insert(export.name.to_string(), export.index);
                        export_names.insert(export.index, export.name.to_string());
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let function_index = current_function_index;
                current_function_index = current_function_index.saturating_add(1);
                let summary = analyze_function(
                    function_index,
                    export_names.get(&function_index).cloned(),
                    body,
                )?;
                functions.push(summary);
            }
            _ => {}
        }
    }

    Ok(ParsedModule {
        summary: ModuleSummary { functions },
        exported_functions,
    })
}

fn analyze_function(
    index: u32,
    export_name: Option<String>,
    body: wasmparser::FunctionBody<'_>,
) -> Result<FunctionSummary> {
    let mut reader = body
        .get_operators_reader()
        .map_err(|e| FormalVerificationError::WasmParse(e.to_string()))?;
    let mut ops = Vec::new();
    let mut unsupported_ops = Vec::new();

    while !reader.eof() {
        let (op, offset) = reader
            .read_with_offset()
            .map_err(|e| FormalVerificationError::WasmParse(e.to_string()))?;
        if let Some(reduced) = reduce_operator(&op) {
            ops.push(reduced);
        } else {
            unsupported_ops.push(format!("{offset}: {op:?}"));
        }
    }

    reader
        .finish()
        .map_err(|e| FormalVerificationError::WasmParse(e.to_string()))?;

    let program = build_program(&ops)?;

    Ok(FunctionSummary {
        index,
        export_name,
        op_count: ops.len(),
        program,
        unsupported_ops,
    })
}

fn reduce_operator(op: &Operator<'_>) -> Option<Op> {
    use Operator::*;

    Some(match op {
        Unreachable => Op::Unreachable,
        Nop => Op::Nop,
        Drop => Op::Drop,
        Select => Op::Select,
        Return => Op::Return,
        LocalGet { local_index } => Op::LocalGet(*local_index),
        LocalSet { local_index } => Op::LocalSet(*local_index),
        LocalTee { local_index } => Op::LocalTee(*local_index),
        GlobalGet { global_index } => Op::GlobalGet(*global_index),
        GlobalSet { global_index } => Op::GlobalSet(*global_index),
        I32Const { value } => Op::I32Const(*value),
        I64Const { value } => Op::I64Const(*value),
        I32Eqz => Op::I32Eqz,
        I64Eqz => Op::I64Eqz,
        I32Add => Op::I32Add,
        I32Sub => Op::I32Sub,
        I32Mul => Op::I32Mul,
        I32DivS => Op::I32DivS,
        I32DivU => Op::I32DivU,
        I32RemS => Op::I32RemS,
        I32RemU => Op::I32RemU,
        I32And => Op::I32And,
        I32Or => Op::I32Or,
        I32Xor => Op::I32Xor,
        I32Shl => Op::I32Shl,
        I32ShrS => Op::I32ShrS,
        I32ShrU => Op::I32ShrU,
        I64Add => Op::I64Add,
        I64Sub => Op::I64Sub,
        I64Mul => Op::I64Mul,
        I64DivS => Op::I64DivS,
        I64DivU => Op::I64DivU,
        I64RemS => Op::I64RemS,
        I64RemU => Op::I64RemU,
        I64And => Op::I64And,
        I64Or => Op::I64Or,
        I64Xor => Op::I64Xor,
        I64Shl => Op::I64Shl,
        I64ShrS => Op::I64ShrS,
        I64ShrU => Op::I64ShrU,
        I32Eq => Op::EqI32,
        I64Eq => Op::EqI64,
        I32Ne => Op::NeI32,
        I64Ne => Op::NeI64,
        I32LtS => Op::LtSI32,
        I64LtS => Op::LtSI64,
        I32LeS => Op::LeSI32,
        I64LeS => Op::LeSI64,
        I32GtS => Op::GtSI32,
        I64GtS => Op::GtSI64,
        I32GeS => Op::GeSI32,
        I64GeS => Op::GeSI64,
        Block { .. } => Op::Block,
        Loop { .. } => Op::Loop,
        If { .. } => Op::If,
        Else => Op::Else,
        End => Op::End,
        Br { relative_depth } => Op::Br(*relative_depth),
        BrIf { relative_depth } => Op::BrIf(*relative_depth),
        Call { function_index } => Op::Call(*function_index),
        CallIndirect { .. } => Op::CallIndirect,
        I32Load { .. } | I64Load { .. } | F32Load { .. } | F64Load { .. } => {
            Op::LoadFreshInt(format!("{op:?}"))
        }
        I32Store { .. }
        | I64Store { .. }
        | F32Store { .. }
        | F64Store { .. }
        | MemoryGrow { .. }
        | MemorySize { .. } => return Some(Op::Nop),
        _ => return None,
    })
}

fn build_program(ops: &[Op]) -> Result<Program> {
    fn push_node(stack: &mut Vec<Frame>, node: Node) {
        if let Some(top) = stack.last_mut() {
            top.push(node);
        }
    }

    let mut stack = vec![Frame {
        kind: FrameKind::Root,
        then_nodes: Vec::new(),
        else_nodes: Vec::new(),
        in_else: false,
    }];

    for op in ops {
        match op {
            Op::Block => stack.push(Frame {
                kind: FrameKind::Block,
                then_nodes: Vec::new(),
                else_nodes: Vec::new(),
                in_else: false,
            }),
            Op::Loop => stack.push(Frame {
                kind: FrameKind::Loop,
                then_nodes: Vec::new(),
                else_nodes: Vec::new(),
                in_else: false,
            }),
            Op::If => stack.push(Frame {
                kind: FrameKind::If,
                then_nodes: Vec::new(),
                else_nodes: Vec::new(),
                in_else: false,
            }),
            Op::Else => {
                let top = stack.last_mut().ok_or_else(|| {
                    FormalVerificationError::Symbolic("unexpected else".to_string())
                })?;
                if top.kind != FrameKind::If {
                    return Err(FormalVerificationError::Symbolic(
                        "else without matching if".to_string(),
                    ));
                }
                if top.in_else {
                    return Err(FormalVerificationError::Symbolic(
                        "duplicate else for if block".to_string(),
                    ));
                }
                top.in_else = true;
            }
            Op::End => {
                if stack.len() == 1 {
                    return Err(FormalVerificationError::Symbolic(
                        "unexpected end without open block".to_string(),
                    ));
                }
                let frame = stack.pop().unwrap();
                let node = match frame.kind {
                    FrameKind::Block => Node::Block(frame.then_nodes),
                    FrameKind::Loop => Node::Loop(frame.then_nodes),
                    FrameKind::If => Node::If {
                        then_nodes: frame.then_nodes,
                        else_nodes: frame.else_nodes,
                    },
                    FrameKind::Root => unreachable!(),
                };
                push_node(&mut stack, node);
            }
            other => {
                let node = Node::Op(other.clone());
                push_node(&mut stack, node);
            }
        }
    }

    if stack.len() != 1 {
        return Err(FormalVerificationError::Symbolic(
            "unterminated control frame in wasm function".to_string(),
        ));
    }
    let root = stack
        .pop()
        .ok_or_else(|| FormalVerificationError::Symbolic("empty program".to_string()))?;
    Ok(Program {
        nodes: root.then_nodes,
    })
}
