'use client'

import { useEffect, useState } from 'react'

export default function LedgerRoutePanel() {
  const [ledger, setLedger] = useState({ latestLedger: 0, latestLedgerCloseMs: 0 })
  const [nowMs, setNowMs] = useState(0)

  useEffect(() => {
    let cancelled = false

    async function refresh() {
      try {
        const response = await fetch('/api/ledger/latest', { cache: 'no-store' })
        if (!response.ok || cancelled) return
        const payload = await response.json()
        if (!cancelled) setLedger(payload)
      } catch {
        // Ignore transient failures.
      }
    }

    refresh()
    const id = window.setInterval(() => {
      setNowMs(Date.now())
      refresh()
    }, 2500)
    return () => {
      cancelled = true
      window.clearInterval(id)
    }
  }, [])

  const ageMs = Math.max(0, nowMs - ledger.latestLedgerCloseMs)

  return (
    <>
      <p>Latest ledger: {ledger.latestLedger}</p>
      <p>Closed: {ledger.latestLedgerCloseMs > 0 ? new Date(ledger.latestLedgerCloseMs).toISOString() : 'pending'}</p>
      <p>Payload age estimate: {ageMs} ms</p>
      {ageMs > 5000 ? <p>State: STALE - force network refresh required.</p> : <p>State: FRESH</p>}
    </>
  )
}
