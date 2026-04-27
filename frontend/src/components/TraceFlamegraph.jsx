import { useEffect, useMemo, useRef, useState } from 'react'
import * as d3 from 'd3'
import { flamegraph as createFlamegraph } from 'd3-flame-graph'
import 'd3-flame-graph/dist/d3-flamegraph.css'
import './TraceFlamegraph.css'
import { traceEventsToFlamegraph } from '../tracing/traceTree'

function colorForDepth(depth) {
    const normalized = Math.min(1, Math.max(0, depth / 8))
    return d3.interpolateTurbo(normalized)
}

export function TraceFlamegraph({ traceData, title = 'Session Flamegraph', height = 360 }) {
    const containerRef = useRef(null)
    const [selectedNode, setSelectedNode] = useState(null)

    const normalizedData = useMemo(() => {
        if (!traceData) {
            return { name: 'session', value: 0, children: [] }
        }

        if (traceData?.children) {
            return traceData
        }

        return traceEventsToFlamegraph(traceData)
    }, [traceData])

    useEffect(() => {
        const container = containerRef.current
        if (!container || !normalizedData) {
            return undefined
        }

        container.innerHTML = ''
        const width = Math.max(container.clientWidth, 520)

        const chart = createFlamegraph()
            .width(width)
            .height(height)
            .cellHeight(22)
            .minFrameSize(2)
            .transitionDuration(300)
            .sort(true)
            .title('')
            .selfValue(false)
            .setColorMapper((entry, originalColor) => {
                if (!entry || typeof entry.depth !== 'number') {
                    return originalColor
                }
                return colorForDepth(entry.depth)
            })
            .onClick((entry) => {
                if (!entry?.data) {
                    setSelectedNode(null)
                    return
                }
                setSelectedNode({
                    name: entry.data.name,
                    value: Number(entry.value ?? entry.data.value ?? 0),
                    depth: Number(entry.depth ?? 0),
                })
            })

        d3.select(container).datum(normalizedData).call(chart)

        const resizeObserver = new ResizeObserver(() => {
            const nextWidth = Math.max(container.clientWidth, 520)
            chart.width(nextWidth)
            d3.select(container).call(chart)
        })

        resizeObserver.observe(container)

        return () => {
            resizeObserver.disconnect()
            container.innerHTML = ''
        }
    }, [normalizedData, height])

    return (
        <section className="trace-flamegraph">
            <header className="trace-flamegraph-header">
                <h3>{title}</h3>
                <p>Click a frame to zoom and inspect cumulative render cost.</p>
            </header>

            <div ref={containerRef} className="trace-flamegraph-canvas" />

            <footer className="trace-flamegraph-footer">
                {selectedNode ? (
                    <p>
                        <strong>{selectedNode.name}</strong> | {selectedNode.value.toFixed(3)} ms total | depth {selectedNode.depth}
                    </p>
                ) : (
                    <p>Select a frame to inspect timing details.</p>
                )}
            </footer>
        </section>
    )
}
