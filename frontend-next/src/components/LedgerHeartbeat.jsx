'use client'

import { useEffect } from 'react'

const POLL_INTERVAL_MS = 2500

export default function LedgerHeartbeat() {
  useEffect(() => {
    let timerId
    let stopped = false

    const publish = async () => {
      try {
        const response = await fetch('/api/ledger/latest', { cache: 'no-store' })
        if (!response.ok) return
        const payload = await response.json()
        if (!navigator.serviceWorker?.controller) return
        navigator.serviceWorker.controller.postMessage({
          type: 'LEDGER_UPDATE',
          latestLedgerCloseMs: payload.latestLedgerCloseMs
        })
      } catch {
        // Ignore transient errors.
      } finally {
        if (!stopped) timerId = window.setTimeout(publish, POLL_INTERVAL_MS)
      }
    }

    publish()
    return () => {
      stopped = true
      window.clearTimeout(timerId)
    }
  }, [])

  return null
}
