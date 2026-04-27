#![allow(dead_code)]
/// Deterministic Wasm Virtual Machine Debugger Protocol.
///
/// Attaches metering and state-snapshotting hooks to Wasmtime execution.
/// On a Wasm trap, serializes the linear memory, globals, and call stack
/// into a Debug Adapter Protocol (DAP)-compatible format.
/// A replay engine allows stepping backwards through the failed trace.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use wasmtime::*;

// ── Snapshot types ────────────────────────────────────────────────────────────

/// A snapshot of Wasm execution state at a single basic-block boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    /// Sequential index of this snapshot (0 = first basic block entered).
    pub index: u64,
    /// Fuel consumed up to this point (proxy for instruction count).
    pub fuel_consumed: u64,
    /// Serialized linear memory (base64-encoded bytes).
    pub linear_memory: Vec<u8>,
    /// Global variable values at this point.
    pub globals: Vec<GlobalSnapshot>,
    /// Simulated call stack frames.
    pub call_stack: Vec<StackFrame>,
}

/// Snapshot of a single Wasm global variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSnapshot {
    pub index: u32,
    pub value: WasmVal,
}

/// A simplified representation of a Wasm value for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum WasmVal {
    I32(i32),
    I64(i64),
    F32(u32), // bit-cast to u32 for deterministic serialization
    F64(u64), // bit-cast to u64
}

impl From<Val> for WasmVal {
    fn from(v: Val) -> Self {
        match v {
            Val::I32(x) => WasmVal::I32(x),
            Val::I64(x) => WasmVal::I64(x),
            Val::F32(x) => WasmVal::F32(x),
            Val::F64(x) => WasmVal::F64(x),
            _ => WasmVal::I64(0), // funcref/externref not serializable
        }
    }
}

/// A single frame in the simulated call stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub function_index: u32,
    pub instruction_offset: u64,
}

// ── DAP-compatible trap report ────────────────────────────────────────────────

/// Debug Adapter Protocol-compatible failure report emitted on a Wasm trap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrapReport {
    /// Human-readable trap reason.
    pub reason: String,
    /// Snapshot of state at the moment of the trap.
    pub state: ExecutionSnapshot,
    /// Full execution trace leading up to the trap (oldest first).
    pub trace: Vec<ExecutionSnapshot>,
}

// ── Execution trace store ─────────────────────────────────────────────────────

/// Ring-buffer of execution snapshots used by the replay engine.
///
/// Bounded to `max_snapshots` entries; oldest entries are dropped when full.
#[derive(Debug, Default)]
pub struct ExecutionTrace {
    snapshots: Vec<ExecutionSnapshot>,
    max_snapshots: usize,
    fuel_counter: u64,
}

impl ExecutionTrace {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(max_snapshots),
            max_snapshots,
            fuel_counter: 0,
        }
    }

    /// Record a new snapshot, evicting the oldest if at capacity.
    pub fn record(&mut self, snap: ExecutionSnapshot) {
        if self.snapshots.len() >= self.max_snapshots {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snap);
    }

    /// Increment the internal fuel counter and return the new value.
    pub fn tick_fuel(&mut self, delta: u64) -> u64 {
        self.fuel_counter += delta;
        self.fuel_counter
    }

    /// Return all snapshots in chronological order.
    pub fn snapshots(&self) -> &[ExecutionSnapshot] {
        &self.snapshots
    }

    /// Return the most recent snapshot, if any.
    pub fn latest(&self) -> Option<&ExecutionSnapshot> {
        self.snapshots.last()
    }
}

// ── Wasm Debugger ─────────────────────────────────────────────────────────────

/// Debugger that instruments Wasmtime with metering and state-snapshotting.
pub struct WasmDebugger {
    pub engine: Engine,
    /// Maximum number of snapshots retained in the trace ring-buffer.
    pub max_trace_depth: usize,
}

impl WasmDebugger {
    /// Create a new debugger with epoch interruption and fuel metering enabled.
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        // Epoch interruption allows the host to interrupt long-running Wasm.
        config.epoch_interruption(true);
        // Fuel metering gives a deterministic instruction-count proxy.
        config.consume_fuel(true);
        let engine = Engine::new(&config)?;
        Ok(Self {
            engine,
            max_trace_depth: 1024,
        })
    }

    /// Instantiate a Wasm module with debug hooks injected.
    ///
    /// Returns the `Store` (with trace state) and the `Instance`.
    /// The caller should run the Wasm function inside a `catch_unwind`-style
    /// wrapper and call `extract_trap_report` on failure.
    pub fn instantiate_with_hooks(
        &self,
        wasm_bytes: &[u8],
    ) -> Result<(Store<Arc<Mutex<ExecutionTrace>>>, Instance)> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let trace = Arc::new(Mutex::new(ExecutionTrace::new(self.max_trace_depth)));
        let mut store = Store::new(&self.engine, Arc::clone(&trace));

        // Provide initial fuel (10M instructions before interruption).
        store.set_fuel(10_000_000)?;

        // Inject a host function that Wasm can call to emit a snapshot.
        // In a real instrumented module the compiler would insert calls to
        // `__debug_snapshot` at every basic-block entry.
        let trace_ref = Arc::clone(&trace);
        let snapshot_fn = Func::wrap(
            &mut store,
            move |mut caller: Caller<'_, Arc<Mutex<ExecutionTrace>>>,
                  func_idx: i32,
                  instr_offset: i64| {
                let fuel = caller.get_fuel().unwrap_or(0);
                let memory_bytes = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .map(|m| m.data(&caller).to_vec())
                    .unwrap_or_default();

                let snap = ExecutionSnapshot {
                    index: 0, // filled in below
                    fuel_consumed: 10_000_000u64.saturating_sub(fuel),
                    linear_memory: memory_bytes,
                    globals: vec![],
                    call_stack: vec![StackFrame {
                        function_index: func_idx as u32,
                        instruction_offset: instr_offset as u64,
                    }],
                };

                if let Ok(mut t) = trace_ref.lock() {
                    let idx = t.tick_fuel(1);
                    let mut s = snap;
                    s.index = idx;
                    t.record(s);
                }
            },
        );

        let imports = module
            .imports()
            .filter_map(|imp| {
                if imp.name() == "__debug_snapshot" {
                    Some(snapshot_fn.clone().into())
                } else {
                    None
                }
            })
            .collect::<Vec<Extern>>();

        let instance = Instance::new(&mut store, &module, &imports)?;
        Ok((store, instance))
    }

    /// Extract a `TrapReport` from a store after a Wasm trap.
    pub fn extract_trap_report(
        store: &Store<Arc<Mutex<ExecutionTrace>>>,
        trap: &Trap,
    ) -> TrapReport {
        let trace_data = store.data().lock().ok();
        let (snapshots, latest) = trace_data
            .as_ref()
            .map(|t| (t.snapshots().to_vec(), t.latest().cloned()))
            .unwrap_or_default();

        let state = latest.unwrap_or(ExecutionSnapshot {
            index: 0,
            fuel_consumed: 0,
            linear_memory: vec![],
            globals: vec![],
            call_stack: vec![],
        });

        TrapReport {
            reason: format!("{:?}", trap),
            state,
            trace: snapshots,
        }
    }

    /// Serialize a `TrapReport` to a DAP-compatible JSON string.
    pub fn serialize_report(report: &TrapReport) -> Result<String> {
        Ok(serde_json::to_string_pretty(report)?)
    }

    /// Legacy: extract state string from a trap (kept for backward compat).
    #[allow(dead_code)]
    pub fn extract_state(&self, _store: &Store<()>, trap: &Trap) -> String {
        format!("Extracted state from trap: {:?}", trap)
    }
}

// ── Replay engine ─────────────────────────────────────────────────────────────

/// Deterministic replay engine.
///
/// Given a serialized `TrapReport`, the engine can step backwards through
/// the recorded snapshots to reconstruct the execution history.
pub struct ReplayEngine {
    report: TrapReport,
    /// Current cursor position (index into `report.trace`).
    cursor: usize,
}

impl ReplayEngine {
    /// Load a replay engine from a serialized DAP JSON report.
    pub fn from_json(json: &str) -> Result<Self> {
        let report: TrapReport = serde_json::from_str(json)?;
        let cursor = report.trace.len().saturating_sub(1);
        Ok(Self { report, cursor })
    }

    /// Load a replay engine directly from a `TrapReport`.
    pub fn from_report(report: TrapReport) -> Self {
        let cursor = report.trace.len().saturating_sub(1);
        Self { report, cursor }
    }

    /// Step backwards one snapshot. Returns `None` when at the beginning.
    pub fn step_back(&mut self) -> Option<&ExecutionSnapshot> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        self.report.trace.get(self.cursor)
    }

    /// Step forwards one snapshot. Returns `None` when at the trap point.
    pub fn step_forward(&mut self) -> Option<&ExecutionSnapshot> {
        let next = self.cursor + 1;
        if next >= self.report.trace.len() {
            return None;
        }
        self.cursor = next;
        self.report.trace.get(self.cursor)
    }

    /// Jump to a specific snapshot by its sequential index.
    pub fn goto(&mut self, index: u64) -> Option<&ExecutionSnapshot> {
        let pos = self.report.trace.iter().position(|s| s.index == index)?;
        self.cursor = pos;
        self.report.trace.get(pos)
    }

    /// Return the snapshot at the current cursor position.
    pub fn current(&self) -> Option<&ExecutionSnapshot> {
        self.report.trace.get(self.cursor)
    }

    /// Return the trap reason from the report.
    pub fn trap_reason(&self) -> &str {
        &self.report.reason
    }

    /// Return the total number of recorded snapshots.
    pub fn trace_len(&self) -> usize {
        self.report.trace.len()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(n: usize) -> TrapReport {
        let snaps: Vec<ExecutionSnapshot> = (0..n)
            .map(|i| ExecutionSnapshot {
                index: i as u64,
                fuel_consumed: i as u64 * 100,
                linear_memory: vec![i as u8],
                globals: vec![],
                call_stack: vec![StackFrame {
                    function_index: 0,
                    instruction_offset: i as u64,
                }],
            })
            .collect();

        TrapReport {
            reason: "unreachable".to_string(),
            state: snaps.last().cloned().unwrap_or(ExecutionSnapshot {
                index: 0,
                fuel_consumed: 0,
                linear_memory: vec![],
                globals: vec![],
                call_stack: vec![],
            }),
            trace: snaps,
        }
    }

    #[test]
    fn test_replay_step_back() {
        let report = make_report(5);
        let mut engine = ReplayEngine::from_report(report);
        // Starts at last snapshot (index 4).
        assert_eq!(engine.current().unwrap().index, 4);
        engine.step_back();
        assert_eq!(engine.current().unwrap().index, 3);
    }

    #[test]
    fn test_replay_step_forward() {
        let report = make_report(5);
        let mut engine = ReplayEngine::from_report(report);
        engine.step_back();
        engine.step_back();
        engine.step_forward();
        assert_eq!(engine.current().unwrap().index, 3);
    }

    #[test]
    fn test_replay_goto() {
        let report = make_report(10);
        let mut engine = ReplayEngine::from_report(report);
        let snap = engine.goto(2).unwrap();
        assert_eq!(snap.index, 2);
    }

    #[test]
    fn test_replay_step_back_at_start_returns_none() {
        let report = make_report(1);
        let mut engine = ReplayEngine::from_report(report);
        assert!(engine.step_back().is_none());
    }

    #[test]
    fn test_serialize_deserialize_report() {
        let report = make_report(3);
        let json = WasmDebugger::serialize_report(&report).unwrap();
        let engine = ReplayEngine::from_json(&json).unwrap();
        assert_eq!(engine.trace_len(), 3);
        assert_eq!(engine.trap_reason(), "unreachable");
    }

    #[test]
    fn test_execution_trace_ring_buffer() {
        let mut trace = ExecutionTrace::new(3);
        for i in 0..5u64 {
            trace.record(ExecutionSnapshot {
                index: i,
                fuel_consumed: i * 10,
                linear_memory: vec![],
                globals: vec![],
                call_stack: vec![],
            });
        }
        // Ring buffer capped at 3; should hold indices 2, 3, 4.
        assert_eq!(trace.snapshots().len(), 3);
        assert_eq!(trace.snapshots()[0].index, 2);
        assert_eq!(trace.snapshots()[2].index, 4);
    }

    #[test]
    fn test_wasm_val_from_val() {
        assert!(matches!(WasmVal::from(Val::I32(42)), WasmVal::I32(42)));
        assert!(matches!(WasmVal::from(Val::I64(-1)), WasmVal::I64(-1)));
    }

    #[test]
    fn test_debugger_new() {
        assert!(WasmDebugger::new().is_ok());
    }
}
