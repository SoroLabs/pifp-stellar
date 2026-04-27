function ensureChild(parent, name) {
    if (!Array.isArray(parent.children)) {
        parent.children = []
    }

    let child = parent.children.find((entry) => entry.name === name)
    if (!child) {
        child = { name, value: 0, children: [] }
        parent.children.push(child)
    }

    return child
}

export function traceEventsToFlamegraph(traceData) {
    const events = Array.isArray(traceData) ? traceData : traceData?.events || []
    const root = { name: 'session', value: 0, children: [] }

    for (const event of events) {
        const duration = Number(event.d ?? event.actualDuration ?? 0)
        if (!Number.isFinite(duration) || duration <= 0) {
            continue
        }

        root.value += duration

        const path = String(event.id || 'unknown')
            .split('>')
            .map((segment) => segment.trim())
            .filter(Boolean)

        if (path.length === 0) {
            const unknownNode = ensureChild(root, 'unknown')
            unknownNode.value += duration
            continue
        }

        let cursor = root
        for (const segment of path) {
            const child = ensureChild(cursor, segment)
            child.value += duration
            cursor = child
        }
    }

    return root
}
