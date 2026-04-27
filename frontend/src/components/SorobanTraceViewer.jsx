import { useMemo, useState } from 'react'
import { extractSourceIdentifier, findFirstSourceNode, parseSorobanTrace } from '../utils/traceParser.ts'
import './SorobanTraceViewer.css'

function renderNode(node) {
  return (
    <details key={node.id} className={node.panic ? 'trace-node panic' : 'trace-node'} open={node.panic}>
      <summary>
        <span>{node.text}</span>
        {node.location && node.lineNumber ? (
          <small>{`${node.location}:${node.lineNumber}`}</small>
        ) : null}
      </summary>
      {node.children.length > 0 && (
        <div className="trace-children">
          {node.children.map((child) => renderNode(child))}
        </div>
      )}
    </details>
  )
}

function SorobanTraceViewer() {
  const [trace, setTrace] = useState('error: panic at contracts/soroban_lib.rs:42\n  0: contract::execute(input=...)\n  1: env::invoke')
  const [parsed, setParsed] = useState(parseSorobanTrace(trace))
  const [source, setSource] = useState('')
  const [sourceFetchError, setSourceFetchError] = useState('')

  const sourceId = useMemo(() => extractSourceIdentifier(trace), [trace])
  const highlightedNode = useMemo(() => findFirstSourceNode(parsed), [parsed])

  const loadSource = async () => {
    if (!sourceId) {
      setSourceFetchError('No contract source identifier found in the trace.')
      return
    }
    setSourceFetchError('')
    try {
      const sourceApi = import.meta.env.VITE_SOURCE_API_URL || 'http://localhost:8080/source'
      const response = await fetch(`${sourceApi}?id=${encodeURIComponent(sourceId)}`)
      if (!response.ok) {
        throw new Error(`Source fetch failed: ${response.status}`)
      }
      const payload = await response.text()
      setSource(payload)
    } catch (err) {
      setSourceFetchError(err.message || 'Failed to fetch source')
    }
  }

  const handleParse = () => {
    setParsed(parseSorobanTrace(trace))
  }

  const displayedSource = useMemo(() => {
    if (!source || !highlightedNode?.lineNumber) return source
    const lines = source.split(/\r?\n/)
    const start = Math.max(0, highlightedNode.lineNumber - 4)
    const end = Math.min(lines.length, highlightedNode.lineNumber + 2)
    return lines.slice(start, end).map((line, index) => {
      const lineNumber = start + index + 1
      const marker = lineNumber === highlightedNode.lineNumber ? '▶ ' : '  '
      return `${marker}${lineNumber.toString().padStart(3, ' ')} | ${line}`
    }).join('\n')
  }, [source, highlightedNode])

  return (
    <section className="trace-panel" aria-label="Soroban trace parser">
      <div className="trace-heading">
        <div>
          <h2>Soroban Error Trace Parser</h2>
          <p>Tokenizes raw VM traces into a navigable call stack and overlays verified source lines when available.</p>
        </div>
      </div>
      <div className="trace-controls">
        <textarea
          aria-label="Paste raw Soroban trace"
          value={trace}
          onChange={(e) => setTrace(e.target.value)}
        />
        <div className="trace-actions">
          <button type="button" onClick={handleParse}>Parse Trace</button>
          <button type="button" onClick={loadSource} disabled={!sourceId}>Fetch Verified Source</button>
        </div>
        {sourceFetchError && <p className="trace-error">{sourceFetchError}</p>}
      </div>
      <div className="trace-body">
        <div className="trace-tree">
          <h3>Parsed Trace</h3>
          {renderNode(parsed)}
        </div>
        <div className="trace-source">
          <h3>Verified Source Snippet</h3>
          <pre>{displayedSource || 'Source not loaded yet.'}</pre>
        </div>
      </div>
    </section>
  )
}

export default SorobanTraceViewer
