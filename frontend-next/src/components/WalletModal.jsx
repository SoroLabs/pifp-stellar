'use client'

import React from 'react'
import Modal from './Modal'
import { useApp } from '@/context/AppContext'

export default function WalletModal() {
  const { isWalletModalOpen, setWalletModalOpen, connectWallet } = useApp()

  const wallets = [
    {
      name: 'Freighter Wallet',
      type: 'Freighter',
      badge: 'Popular',
      description: 'Official browser extension wallet for Stellar & Soroban smart contracts.',
      color: '#3a82f6',
      icon: (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <rect x="2" y="4" width="20" height="16" rx="3"/>
          <path d="M7 10h4M7 14h2"/>
          <circle cx="16" cy="12" r="2" fill="currentColor"/>
        </svg>
      )
    },
    {
      name: 'Albedo Signer',
      type: 'Albedo',
      badge: 'Web',
      description: 'Web-based delegated signer without browser extensions required.',
      color: '#8b5cf6',
      icon: (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="9"/>
          <path d="M12 3v18M3 12h18"/>
        </svg>
      )
    },
    {
      name: 'Instant Testnet Keypair',
      type: 'Demo Keypair',
      badge: 'Quick Test',
      description: 'Instant pre-funded Stellar Testnet account (3,820 XLM) for rapid testing.',
      color: '#10b981',
      icon: (
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
        </svg>
      )
    }
  ]

  return (
    <Modal
      isOpen={isWalletModalOpen}
      onClose={() => setWalletModalOpen(false)}
      title="Connect Stellar Wallet"
      maxWidth="460px"
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <p className="muted" style={{ margin: 0, marginBottom: '0.75rem', fontSize: '0.9rem', lineHeight: 1.5 }}>
          Select a wallet to interact with Soroban smart contracts, deploy impact projects, and authorize verifiable donations on the Stellar ledger.
        </p>

        {wallets.map(w => (
          <button
            key={w.type}
            className="btn glass-panel"
            onClick={() => connectWallet(w.type)}
            style={{
              padding: '1.1rem',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'flex-start',
              gap: '1rem',
              textAlign: 'left',
              width: '100%',
              borderRadius: '12px',
              border: '1px solid var(--panel-border)',
              background: 'rgba(255, 255, 255, 0.03)',
              cursor: 'pointer',
              transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)'
            }}
            onMouseEnter={e => {
              e.currentTarget.style.transform = 'translateY(-2px)'
              e.currentTarget.style.borderColor = 'var(--accent)'
              e.currentTarget.style.background = 'rgba(0, 210, 255, 0.06)'
            }}
            onMouseLeave={e => {
              e.currentTarget.style.transform = 'none'
              e.currentTarget.style.borderColor = 'var(--panel-border)'
              e.currentTarget.style.background = 'rgba(255, 255, 255, 0.03)'
            }}
          >
            <div style={{
              width: '44px',
              height: '44px',
              borderRadius: '10px',
              background: `rgba(${w.color === '#3a82f6' ? '58, 130, 246' : w.color === '#8b5cf6' ? '139, 92, 246' : '16, 185, 129'}, 0.15)`,
              color: w.color,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: 0
            }}>
              {w.icon}
            </div>

            <div style={{ flex: 1 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '2px' }}>
                <span style={{ fontWeight: 700, fontSize: '1rem', color: '#fff' }}>{w.name}</span>
                <span className="pill" style={{ fontSize: '0.7rem', padding: '0.1rem 0.5rem' }}>
                  {w.badge}
                </span>
              </div>
              <div style={{ fontSize: '0.8rem', color: 'var(--muted)', lineHeight: 1.3 }}>
                {w.description}
              </div>
            </div>

            <div style={{ color: 'var(--muted)', fontSize: '1.2rem' }}>→</div>
          </button>
        ))}

        <div style={{
          marginTop: '0.5rem',
          padding: '0.85rem',
          borderRadius: '8px',
          background: 'rgba(0, 210, 255, 0.05)',
          border: '1px solid rgba(0, 210, 255, 0.15)',
          fontSize: '0.8rem',
          color: 'var(--muted)',
          display: 'flex',
          gap: '8px',
          alignItems: 'center'
        }}>
          <span style={{ color: 'var(--accent)', fontSize: '1rem' }}>ℹ</span>
          <span>PIFP runs on Stellar Soroban Testnet. No real funds required for interaction.</span>
        </div>
      </div>
    </Modal>
  )
}
