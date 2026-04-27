import { Profiler, useMemo } from 'react'
import { createTelemetryCapture } from './telemetryCapture'

const defaultCapture = createTelemetryCapture({
    sampleRate: 0.2,
    minDurationMs: 1.25,
})

export function TelemetryProfiler({ id, children, capture }) {
    const activeCapture = useMemo(() => capture || defaultCapture, [capture])
    return (
        <Profiler id={id} onRender={activeCapture.onRender}>
            {children}
        </Profiler>
    )
}

export function getDefaultTelemetryCapture() {
    return defaultCapture
}
