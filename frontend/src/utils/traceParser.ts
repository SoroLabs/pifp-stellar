export type TraceNode = {
  id: string
  text: string
  location?: string
  lineNumber?: number
  children: TraceNode[]
  panic?: boolean
}

const ENTRY_RE = /^(?:#?\d+[:\)]?\s*)?(?<entry>.+?)\s*(?:at\s+(?<location>[^:\s]+?):(?<line>\d+))?$/
const SOURCE_ID_RE = /contract\s*[:=]\s*([A-Za-z0-9_-]+)/i

export function parseSorobanTrace(trace: string) {
  const lines = trace.replace(/\r/g, '').split('\n')
  const root: TraceNode = { id: 'root', text: 'Soroban error trace', children: [] }
  const stack: Array<{ node: TraceNode; indent: number }> = [{ node: root, indent: -1 }]

  for (let index = 0; index < lines.length; index += 1) {
    const rawLine = lines[index]
    if (!rawLine.trim()) continue
    const indent = rawLine.search(/\S|$/)
    const trimmed = rawLine.trim()
    const panic = /panic|error|failed|abort/i.test(trimmed)
    const entryMatch = ENTRY_RE.exec(trimmed)
    const text = entryMatch?.groups?.entry?.trim() || trimmed
    const location = entryMatch?.groups?.location
    const lineNumber = entryMatch?.groups?.line ? Number(entryMatch.groups.line) : undefined

    const node: TraceNode = {
      id: `trace-${index}`,
      text,
      location,
      lineNumber,
      children: [],
      panic,
    }

    while (stack.length > 0 && indent <= stack[stack.length - 1].indent) {
      stack.pop()
    }
    stack[stack.length - 1].node.children.push(node)
    stack.push({ node, indent })
  }

  return root
}

export function extractSourceIdentifier(trace: string) {
  const match = SOURCE_ID_RE.exec(trace)
  return match ? match[1] : undefined
}

export function findFirstSourceNode(node: TraceNode): TraceNode | undefined {
  if (node.location && node.lineNumber) return node
  for (const child of node.children) {
    const match = findFirstSourceNode(child)
    if (match) return match
  }
  return undefined
}
