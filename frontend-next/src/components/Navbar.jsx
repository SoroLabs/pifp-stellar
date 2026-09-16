'use client'

import Link from 'next/link'
import { useState, useEffect } from 'react'
import { useApp } from '@/context/AppContext'

export default function Navbar() {
  const { wallet, setWalletModalOpen, setCreateModalOpen, disconnectWallet, ledger, pulse } = useApp()
  const [isDropdownOpen, setDropdownOpen] = useState(false)
  const latestLedger = ledger?.latestLedger || 100000

  return (
    <header style={{
      position: 'fixed',
      top: 0,
      left: 0,
      right: 0,
      height: '76px',
      zIndex: 1000,
      background: 'rgba(5, 11, 20, 0.85)',
      backdropFilter: 'blur(16px)',
      WebkitBackdropFilter: 'blur(16px)',
      borderBottom: '1px solid var(--glass-border)'
    }}>
      <div className="container" style={{
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between'
      }}>
        {/* Brand & Nav items */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '2.5rem' }}>
          <Link href="/" style={{
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
            textDecoration: 'none'
          }}>
            <div style={{
              width: '36px',
              height: '36px',
              borderRadius: '10px',
              background: 'linear-gradient(135deg, var(--accent), var(--primary))',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#050b14',
              fontWeight: 900,
              fontSize: '1.2rem',
              boxShadow: '0 0 15px var(--accent-glow)'
            }}>
              ✦
            </div>
            <div>
              <span style={{ fontSize: '1.25rem', fontWeight: 800, letterSpacing: '-0.02em', color: '#fff' }}>
                PIFP<span style={{ color: 'var(--accent)' }}>.stellar</span>
              </span>
            </div>
          </Link>

          <nav style={{ display: 'flex', alignItems: 'center', gap: '1.5rem', fontWeight: 500, fontSize: '0.92rem' }}>
            <Link
              href="/explore"
              data-predictive="true"
              style={{
                color: 'var(--text)',
                transition: 'color 0.2s ease',
                display: 'flex',
                alignItems: 'center',
                gap: '6px'
              }}
              onMouseEnter={e => e.currentTarget.style.color = 'var(--accent)'}
              onMouseLeave={e => e.currentTarget.style.color = 'var(--text)'}
            >
              Explore Projects
            </Link>
            <Link
              href="/#protocol-features"
              style={{
                color: 'var(--muted)',
                transition: 'color 0.2s ease'
              }}
              onMouseEnter={e => e.currentTarget.style.color = 'var(--text)'}
              onMouseLeave={e => e.currentTarget.style.color = 'var(--muted)'}
            >
              How It Works
            </Link>
          </nav>
        </div>

        {/* Action Controls */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          {/* Stellar Live Ledger Badge */}
          <div
            className="pill"
            style={{
              background: 'rgba(16, 185, 129, 0.08)',
              borderColor: pulse ? 'var(--ok)' : 'rgba(16, 185, 129, 0.2)',
              transition: 'all 0.3s ease',
              padding: '0.35rem 0.8rem',
              display: 'none',
              fontSize: '0.8rem'
            }}
            id="nav-ledger-pill"
          >
            <span
              className="ok-dot"
              style={{
                transform: pulse ? 'scale(1.4)' : 'scale(1)',
                transition: 'transform 0.3s'
              }}
            />
            <span style={{ color: 'var(--muted)' }}>Ledger:</span>
            <span style={{ color: '#fff', fontWeight: 700 }}>#{latestLedger}</span>
          </div>

          {/* Launch Project CTA */}
          <button
            onClick={() => setCreateModalOpen(true)}
            className="btn btn-outline"
            style={{
              padding: '0.55rem 1.1rem',
              fontSize: '0.88rem',
              display: 'flex',
              alignItems: 'center',
              gap: '6px'
            }}
          >
            <span style={{ fontSize: '1.1rem', lineHeight: 1 }}>+</span>
            <span>Create Project</span>
          </button>

          {/* Wallet Button */}
          {wallet.isConnected ? (
            <div style={{ position: 'relative' }}>
              <button
                onClick={() => setDropdownOpen(!isDropdownOpen)}
                className="btn glass-panel"
                style={{
                  padding: '0.45rem 0.9rem',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '10px',
                  borderRadius: '999px',
                  border: '1px solid var(--panel-border)',
                  background: 'rgba(16, 25, 48, 0.8)'
                }}
              >
                <div style={{
                  background: 'rgba(0, 210, 255, 0.15)',
                  color: 'var(--accent)',
                  fontWeight: 700,
                  fontSize: '0.82rem',
                  padding: '0.2rem 0.6rem',
                  borderRadius: '999px'
                }}>
                  {wallet.balance.toLocaleString()} XLM
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <span className="ok-dot" />
                  <span style={{ fontSize: '0.85rem', fontWeight: 600, color: '#fff' }}>
                    {wallet.shortAddress}
                  </span>
                </div>
                <span style={{ fontSize: '0.7rem', color: 'var(--muted)' }}>▼</span>
              </button>

              {/* Dropdown Menu */}
              {isDropdownOpen && (
                <div
                  className="glass-panel animate-fade-in"
                  style={{
                    position: 'absolute',
                    top: 'calc(100% + 8px)',
                    right: 0,
                    width: '240px',
                    padding: '1rem',
                    borderRadius: '12px',
                    background: 'rgba(9, 16, 29, 0.95)',
                    boxShadow: '0 12px 30px rgba(0,0,0,0.6)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.75rem',
                    zIndex: 1001
                  }}
                >
                  <div style={{ borderBottom: '1px solid var(--glass-border)', paddingBottom: '0.5rem' }}>
                    <div className="muted" style={{ fontSize: '0.75rem' }}>Connected Network</div>
                    <div style={{ fontWeight: 600, fontSize: '0.85rem', color: 'var(--accent)' }}>
                      {wallet.network}
                    </div>
                  </div>

                  <div style={{ borderBottom: '1px solid var(--glass-border)', paddingBottom: '0.5rem' }}>
                    <div className="muted" style={{ fontSize: '0.75rem' }}>Signer</div>
                    <div style={{ fontWeight: 600, fontSize: '0.85rem', color: '#fff' }}>
                      {wallet.walletType || 'Freighter'}
                    </div>
                  </div>

                  <button
                    onClick={() => {
                      disconnectWallet()
                      setDropdownOpen(false)
                    }}
                    className="btn"
                    style={{
                      width: '100%',
                      padding: '0.5rem',
                      background: 'rgba(239, 68, 68, 0.1)',
                      color: 'var(--danger)',
                      border: '1px solid rgba(239, 68, 68, 0.2)',
                      fontSize: '0.85rem'
                    }}
                  >
                    Disconnect Wallet
                  </button>
                </div>
              )}
            </div>
          ) : (
            <button
              onClick={() => setWalletModalOpen(true)}
              className="btn btn-accent"
              style={{ padding: '0.55rem 1.3rem', fontSize: '0.88rem' }}
            >
              Connect Wallet
            </button>
          )}
        </div>
      </div>
      <style>{`
        @media (min-width: 860px) {
          #nav-ledger-pill {
            display: inline-flex !important;
          }
        }
      `}</style>
    </header>
  )
}
