import { useMemo, useRef, useState } from 'react'
import './WasmSandbox.css'

const MAX_MEMORY_PAGES = 2
const DEFAULT_GAS = 30000

function base64ToBytes(base64) {
  return Uint8Array.from(atob(base64.trim()), (c) => c.charCodeAt(0))
}

function createImports(memory, logger, gasRef) {
  return {
    env: {
      memory,
      host_log(ptr, len) {
        const bytes = new Uint8Array(memory.buffer, ptr, len)
        const text = new TextDecoder().decode(bytes)
        logger.current((prev) => [...prev, `host_log: ${text}`])
      },
      host_abort() {
        throw new Error('Execution aborted by sandbox host')
      },
      host_gas() {
        gasRef.current -= 1
        if (gasRef.current <= 0) {
          throw new Error('Gas limit exceeded')
        }
      },
    },
  }
}

function WasmSandbox() {
  const [moduleBytes, setModuleBytes] = useState('')
  const [instanceInfo, setInstanceInfo] = useState(null)
  const [executionLog, setExecutionLog] = useState([])
  const [memorySnapshot, setMemorySnapshot] = useState('')
  const [status, setStatus] = useState('Ready to load Wasm module.')
  const loggerRef = useRef((fn) => setExecutionLog(fn))
  const gasRef = useRef(DEFAULT_GAS)

  const memory = useMemo(() => new WebAssembly.Memory({ initial: 1, maximum: MAX_MEMORY_PAGES }), [])

  const updateMemorySnapshot = () => {
    const view = new Uint8Array(memory.buffer)
    const chunk = Array.from(view.slice(0, 128))
      .map((byte, index) => `${index.toString().padStart(3, ' ')}: ${byte.toString(16).padStart(2, '0')}`)
      .join('\n')
    setMemorySnapshot(chunk)
  }

  const handleLoad = async () => {
    setExecutionLog([])
    setMemorySnapshot('')
    gasRef.current = DEFAULT_GAS
    try {
      const bytes = base64ToBytes(moduleBytes)
      const module = await WebAssembly.compile(bytes)
      const imports = createImports(memory, loggerRef, gasRef)
      const instance = await WebAssembly.instantiate(module, imports)
      setInstanceInfo({ exports: Object.keys(instance.exports) })
      setStatus('Wasm module loaded successfully.')
      updateMemorySnapshot()
    } catch (error) {
      setStatus(`Load failed: ${error.message}`)
    }
  }

  const handleExecute = async () => {
    setExecutionLog((prev) => [...prev, `Running with ${gasRef.current} gas remaining...`])
    if (!instanceInfo) {
      setStatus('Cannot execute until module is loaded.')
      return
    }
    try {
      const bytes = base64ToBytes(moduleBytes)
      const module = await WebAssembly.compile(bytes)
      const imports = createImports(memory, loggerRef, gasRef)
      const instance = await WebAssembly.instantiate(module, imports)
      if (typeof instance.exports.validate === 'function') {
        const result = instance.exports.validate(1)
        setExecutionLog((prev) => [...prev, `validate(1) => ${result}`])
      } else if (typeof instance.exports.run === 'function') {
        const result = instance.exports.run()
        setExecutionLog((prev) => [...prev, `run() => ${result}`])
      } else {
        setExecutionLog((prev) => [...prev, 'No exported validate/run function found.'])
      }
      setStatus('Execution completed successfully.')
      updateMemorySnapshot()
    } catch (error) {
      setStatus(`Execution failed: ${error.message}`)
    }
  }

  return (
    <section className="wasm-panel" aria-label="Wasm sandbox environment">
      <div className="wasm-heading">
        <div>
          <h2>Browser Wasm Sandbox</h2>
          <p>Strict memory and gas-limited Wasm execution with restricted host imports and debugger state.</p>
        </div>
      </div>
      <div className="wasm-controls">
        <textarea
          aria-label="Paste base64-encoded Wasm module"
          placeholder="Paste base64-encoded Wasm module here"
          value={moduleBytes}
          onChange={(e) => setModuleBytes(e.target.value)}
        />
        <div className="wasm-actions">
          <button type="button" onClick={handleLoad}>Load Module</button>
          <button type="button" onClick={handleExecute} disabled={!moduleBytes}>Execute</button>
        </div>
      </div>
      <div className="wasm-status">
        <span>{status}</span>
        {instanceInfo && <small>Exports: {instanceInfo.exports.join(', ')}</small>}
      </div>
      <div className="wasm-body">
        <div className="wasm-panel-column">
          <h3>Execution Log</h3>
          <pre>{executionLog.length > 0 ? executionLog.join('\n') : 'Logs will appear here.'}</pre>
        </div>
        <div className="wasm-panel-column">
          <h3>Memory Snapshot</h3>
          <pre>{memorySnapshot || 'Memory will show first 128 bytes after load/execution.'}</pre>
        </div>
      </div>
    </section>
  )
}

export default WasmSandbox
