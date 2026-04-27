function roundMs(value) {
    return Math.round(value * 1000) / 1000
}

function supportsRequestIdleCallback() {
    return typeof window !== 'undefined' && typeof window.requestIdleCallback === 'function'
}

function scheduleIdle(fn) {
    if (supportsRequestIdleCallback()) {
        window.requestIdleCallback(fn, { timeout: 1000 })
        return
    }
    setTimeout(fn, 0)
}

function createSessionId() {
    const random = Math.random().toString(36).slice(2, 10)
    return `sess_${Date.now()}_${random}`
}

export function createTelemetryCapture(options = {}) {
    const {
        sessionId = createSessionId(),
        sampleRate = 0.2,
        minDurationMs = 1,
        maxBufferSize = 2000,
        flushIntervalMs = 5000,
        onFlush = () => { },
    } = options

    let buffer = []
    let droppedCount = 0
    let flushScheduled = false

    const flush = () => {
        if (!buffer.length) {
            return
        }

        const payload = {
            sessionId,
            capturedAt: Date.now(),
            droppedCount,
            events: buffer,
        }

        buffer = []
        droppedCount = 0
        onFlush(payload)
    }

    const scheduleFlush = () => {
        if (flushScheduled) {
            return
        }
        flushScheduled = true
        scheduleIdle(() => {
            flushScheduled = false
            flush()
        })
    }

    const intervalId = setInterval(() => {
        scheduleFlush()
    }, flushIntervalMs)

    const onRender = (id, phase, actualDuration, baseDuration, startTime, commitTime) => {
        if (actualDuration < minDurationMs || Math.random() > sampleRate) {
            return
        }

        if (buffer.length >= maxBufferSize) {
            droppedCount += 1
            if ((droppedCount & 31) === 0) {
                scheduleFlush()
            }
            return
        }

        buffer.push({
            id,
            p: phase === 'mount' ? 'm' : 'u',
            d: roundMs(actualDuration),
            b: roundMs(baseDuration),
            s: roundMs(startTime),
            c: roundMs(commitTime),
        })

        if (buffer.length >= maxBufferSize * 0.5) {
            scheduleFlush()
        }
    }

    const snapshot = () => ({
        sessionId,
        queuedEvents: buffer.length,
        droppedCount,
    })

    const stop = () => {
        clearInterval(intervalId)
        flush()
    }

    return {
        sessionId,
        onRender,
        flush,
        stop,
        snapshot,
    }
}
