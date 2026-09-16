'use client'

import React from 'react'
import { useApp } from '@/context/AppContext'

export default function ToastContainer() {
  const { toasts, removeToast } = useApp()

  if (!toasts || toasts.length === 0) return null

  return (
    <aside aria-label="System notifications" style={{
      position: 'fixed',
      top: '84px',
      right: '24px',
      zIndex: 9999,
      display: 'flex',
      flexDirection: 'column',
      gap: '12px',
      maxWidth: '380px',
      width: '100%',
      pointerEvents: 'none'
    }}>
      {toasts.map(toast => (
        <div
          key={toast.id}
          className="glass-panel animate-fade-in"
          style={{
            pointerEvents: 'auto',
            padding: '1rem 1.25rem',
            borderRadius: '12px',
            borderLeft: `4px solid ${
              toast.type === 'error' ? 'var(--danger)' :
              toast.type === 'info' ? 'var(--accent)' : 'var(--ok)'
            }`,
            background: 'rgba(11, 20, 38, 0.95)',
            boxShadow: '0 12px 30px rgba(0,0,0,0.5)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'flex-start',
            gap: '12px'
          }}
        >
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 700, fontSize: '0.95rem', color: '#fff', marginBottom: '2px' }}>
              {toast.title}
            </div>
            <div style={{ fontSize: '0.85rem', color: 'var(--muted)', lineHeight: 1.4 }}>
              {toast.message}
            </div>
          </div>
          <button
            onClick={() => removeToast(toast.id)}
            style={{
              color: 'var(--muted)',
              fontSize: '1.2rem',
              lineHeight: 1,
              padding: '2px 4px',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            ×
          </button>
        </div>
      ))}
    </aside>
  )
}
