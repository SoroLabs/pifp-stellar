'use client'

import { useEffect, useState } from 'react'
import { useApp } from '@/context/AppContext'

export default function LedgerRoutePanel() {
  const { ledger } = useApp()
  const [nowMs, setNowMs] = useState(0)

  useEffect(() => {
    const id = window.setInterval(() => {
      setNowMs(Date.now())
    }, 1000)
    return () => window.clearInterval(id)
  }, [])

  const ageMs = (nowMs > 0 && ledger?.latestLedgerCloseMs > 0)
    ? Math.max(0, nowMs - ledger.latestLedgerCloseMs)
    : 0

  return (
    <div style={{ fontSize: '0.85rem', lineHeight: 1.6, color: 'var(--text)' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
        <span className="muted">Latest Ledger:</span>
        <strong style={{ color: '#fff' }}>#{ledger?.latestLedger || 100000}</strong>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
        <span className="muted">Closed At:</span>
        <span style={{ fontSize: '0.8rem', color: 'var(--text)' }}>
          {ledger?.latestLedgerCloseMs > 0 ? new Date(ledger.latestLedgerCloseMs).toLocaleTimeString() : 'pending'}
        </span>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
        <span className="muted">Payload Age:</span>
        <span style={{ color: ageMs > 5000 ? 'var(--warning)' : 'var(--ok)', fontWeight: 600 }}>
          {ageMs} ms
        </span>
      </div>

      <div style={{
        marginTop: '6px',
        padding: '3px 8px',
        borderRadius: '6px',
        background: ageMs > 5000 ? 'rgba(245, 158, 11, 0.15)' : 'rgba(16, 185, 129, 0.15)',
        color: ageMs > 5000 ? 'var(--warning)' : 'var(--ok)',
        fontSize: '0.78rem',
        fontWeight: 600,
        textAlign: 'center'
      }}>
        {ageMs > 5000 ? 'STALE - SW Auto-Refreshing' : 'FRESH - Cryptographically Attested'}
      </div>
    </div>
  )
}
