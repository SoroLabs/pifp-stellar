'use client'

import React, { useState } from 'react'
import { useParams } from 'next/navigation'
import Link from 'next/link'
import Modal from '@/components/Modal'
import LedgerRoutePanel from '@/components/LedgerRoutePanel'
import { useApp } from '@/context/AppContext'

export default function ProjectDetailsPage() {
  const params = useParams()
  const slug = params?.slug

  const { projects, wallet, setWalletModalOpen, donateToProject, addToast } = useApp()

  const project = projects.find(p => p.id === slug) || projects[0]

  const [donateAmount, setDonateAmount] = useState('100')
  const [isDonateModalOpen, setDonateModalOpen] = useState(false)
  const [donateStep, setDonateStep] = useState('idle') // idle, loading, success
  const [lastTxHash, setLastTxHash] = useState('')

  if (!project) {
    return (
      <div className="container" style={{ padding: '6rem 1.5rem', textAlign: 'center' }}>
        <h2 style={{ fontSize: '2rem', marginBottom: '1rem', color: '#fff' }}>Initiative Not Found</h2>
        <p className="muted" style={{ marginBottom: '2rem' }}>
          The requested project could not be found on the Soroban ledger registry.
        </p>
        <Link href="/explore" className="btn btn-primary">
          Return to Registry
        </Link>
      </div>
    )
  }

  const progress = Math.min(Math.round((project.current / (project.goal || 1)) * 100), 100)

  const handleQuickAdd = (amt) => {
    setDonateAmount(String(amt))
  }

  const handleInitiateDonation = () => {
    if (!wallet.isConnected) {
      setWalletModalOpen(true)
      return
    }
    setDonateModalOpen(true)
  }

  const handleConfirmDonation = () => {
    const amt = Number(donateAmount)
    if (isNaN(amt) || amt <= 0) return

    setDonateStep('loading')

    setTimeout(() => {
      const success = donateToProject(project.id, amt)
      if (success) {
        setLastTxHash(`0x${Array.from({ length: 16 }, () => Math.floor(Math.random() * 16).toString(16)).join('')}`)
        setDonateStep('success')
      } else {
        setDonateStep('idle')
      }
    }, 1200)
  }

  const handleCloseModal = () => {
    setDonateModalOpen(false)
    setTimeout(() => {
      setDonateStep('idle')
    }, 300)
  }

  const handleCopy = (text, label) => {
    navigator.clipboard?.writeText(text)
    addToast('Copied to Clipboard', `${label} copied successfully.`, 'info')
  }

  return (
    <div className="container animate-fade-in" style={{ padding: '2rem 1.5rem 6rem' }}>
      {/* Breadcrumb Navigation */}
      <div style={{ marginBottom: '1.5rem' }}>
        <Link
          href="/explore"
          data-predictive="true"
          style={{
            color: 'var(--muted)',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '8px',
            fontSize: '0.9rem',
            fontWeight: 500,
            transition: 'color 0.2s'
          }}
          onMouseEnter={e => e.currentTarget.style.color = 'var(--accent)'}
          onMouseLeave={e => e.currentTarget.style.color = 'var(--muted)'}
        >
          ← Back to All Initiatives
        </Link>
      </div>

      {/* Hero Banner with Futuristic Circuit Pattern */}
      <div style={{
        height: '240px',
        background: project.gradient || 'linear-gradient(135deg, #0284c7 0%, #0369a1 50%, #075985 100%)',
        borderRadius: '20px',
        marginBottom: '2.5rem',
        position: 'relative',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        padding: '2rem',
        border: '1px solid rgba(255, 255, 255, 0.1)'
      }}>
        {/* SVG circuitry overlay */}
        <svg
          style={{ position: 'absolute', inset: 0, opacity: 0.3, width: '100%', height: '100%', pointerEvents: 'none' }}
          xmlns="http://www.w3.org/2000/svg"
        >
          <defs>
            <pattern id={`detail-grid-${project.id}`} width="30" height="30" patternUnits="userSpaceOnUse">
              <path d="M 30 0 L 0 0 0 30" fill="none" stroke="white" strokeWidth="0.8" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill={`url(#detail-grid-${project.id})`} />
          <circle cx="80%" cy="40%" r="80" fill="rgba(255,255,255,0.18)" filter="blur(30px)" />
        </svg>

        <div style={{ position: 'relative', zIndex: 2, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span className={`pill ${project.category.toLowerCase()}`} style={{
            backdropFilter: 'blur(10px)',
            background: 'rgba(5, 11, 20, 0.8)',
            border: '1px solid rgba(255, 255, 255, 0.2)',
            padding: '0.4rem 1rem',
            fontSize: '0.9rem'
          }}>
            <span className="ok-dot" />
            {project.category} Protocol
          </span>

          <button
            onClick={() => handleCopy(window.location.href, 'Project URL')}
            className="btn glass-panel"
            style={{
              padding: '0.4rem 0.9rem',
              fontSize: '0.82rem',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              color: '#fff',
              background: 'rgba(0,0,0,0.4)'
            }}
          >
            <span>🔗 Share</span>
          </button>
        </div>

        <div style={{ position: 'relative', zIndex: 2 }}>
          <h1 style={{
            fontSize: 'clamp(1.8rem, 3.5vw, 2.8rem)',
            color: '#fff',
            margin: '0 0 0.5rem 0',
            textShadow: '0 2px 10px rgba(0,0,0,0.5)'
          }}>
            {project.title}
          </h1>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', flexWrap: 'wrap', fontSize: '0.85rem', color: 'rgba(255,255,255,0.85)' }}>
            <span>Creator: <code style={{ color: 'var(--accent)', cursor: 'pointer' }} onClick={() => handleCopy(project.creator, 'Creator Address')}>{project.creator.slice(0, 8)}...{project.creator.slice(-4)} 📋</code></span>
            <span>•</span>
            <span>Contract: <code style={{ color: '#fff' }}>{project.contractAddress}</code></span>
          </div>
        </div>
      </div>

      {/* Main Two-Column Layout */}
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 2fr) minmax(320px, 1.2fr)', gap: '2.5rem', alignItems: 'start' }}>
        {/* Left Column: Details & Milestones */}
        <div>
          {/* Executive Overview */}
          <section className="glass-panel" style={{ padding: '2rem', marginBottom: '2rem' }}>
            <h2 style={{ fontSize: '1.4rem', color: '#fff', marginBottom: '1rem' }}>
              Initiative Overview
            </h2>
            <p style={{ fontSize: '1.05rem', color: 'var(--text)', lineHeight: 1.7, margin: '0 0 1.5rem 0' }}>
              {project.description}
            </p>
            <p style={{ fontSize: '0.98rem', color: 'var(--muted)', lineHeight: 1.7, margin: 0 }}>
              {project.longDescription}
            </p>
          </section>

          {/* Soroban Milestones Timeline */}
          <section className="glass-panel" style={{ padding: '2rem', marginBottom: '2rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem', flexWrap: 'wrap', gap: '0.5rem' }}>
              <div>
                <h2 style={{ fontSize: '1.4rem', color: '#fff', margin: 0 }}>
                  Cryptographic Escrow Milestones
                </h2>
                <div className="muted" style={{ fontSize: '0.85rem', marginTop: '4px' }}>
                  Funds release upon {project.oracleThreshold} signature verification
                </div>
              </div>

              <span className="pill funding" style={{ fontSize: '0.8rem' }}>
                {project.milestones.filter(m => m.status === 'completed').length} of {project.milestones.length} Completed
              </span>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
              {project.milestones.map((m, idx) => (
                <div
                  key={m.id || idx}
                  style={{
                    padding: '1.25rem',
                    borderRadius: '12px',
                    border: '1px solid var(--panel-border)',
                    background: m.status === 'completed' ? 'rgba(16, 185, 129, 0.05)' :
                                m.status === 'in-progress' ? 'rgba(0, 210, 255, 0.05)' : 'rgba(255, 255, 255, 0.02)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.75rem'
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '1rem' }}>
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '4px' }}>
                        <span style={{
                          width: '24px',
                          height: '24px',
                          borderRadius: '50%',
                          background: m.status === 'completed' ? 'var(--ok)' :
                                      m.status === 'in-progress' ? 'var(--accent)' : 'rgba(255,255,255,0.2)',
                          color: '#050b14',
                          fontWeight: 800,
                          fontSize: '0.8rem',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center'
                        }}>
                          {idx + 1}
                        </span>
                        <h3 style={{ margin: 0, fontSize: '1.1rem', color: '#fff' }}>{m.title}</h3>
                      </div>
                      <p className="muted" style={{ margin: '0 0 0 32px', fontSize: '0.88rem', lineHeight: 1.5 }}>
                        {m.description}
                      </p>
                    </div>

                    <span className={`pill ${m.status === 'completed' ? 'verified' : m.status === 'in-progress' ? 'funding' : ''}`} style={{ flexShrink: 0 }}>
                      {m.status.toUpperCase()}
                    </span>
                  </div>

                  <div style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    fontSize: '0.82rem',
                    color: 'var(--muted)',
                    borderTop: '1px solid rgba(255, 255, 255, 0.06)',
                    paddingTop: '0.75rem',
                    marginLeft: '32px'
                  }}>
                    <div>
                      Tranche: <strong style={{ color: '#fff' }}>{m.amount.toLocaleString()} XLM</strong> ({m.bps / 100}%)
                    </div>
                    <div>
                      Proof Hash: <code style={{ color: 'var(--accent)' }}>{m.proofHash}</code>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Smart Contract Technical Verification */}
          <section className="glass-panel" style={{ padding: '2rem', marginBottom: '2rem' }}>
            <h2 style={{ fontSize: '1.4rem', color: '#fff', marginBottom: '1.25rem' }}>
              Soroban Protocol Parameters
            </h2>

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1.5rem' }}>
              <div>
                <div className="muted" style={{ fontSize: '0.8rem', marginBottom: '4px' }}>Contract ID</div>
                <div style={{ fontWeight: 600, color: 'var(--accent)', fontSize: '0.9rem' }}>{project.contractAddress}</div>
              </div>

              <div>
                <div className="muted" style={{ fontSize: '0.8rem', marginBottom: '4px' }}>Accepted Token</div>
                <div style={{ fontWeight: 600, color: '#fff', fontSize: '0.9rem' }}>Stellar Native ({project.token})</div>
              </div>

              <div>
                <div className="muted" style={{ fontSize: '0.8rem', marginBottom: '4px' }}>Oracle Consensus</div>
                <div style={{ fontWeight: 600, color: 'var(--ok)', fontSize: '0.9rem' }}>{project.oracleThreshold}</div>
              </div>

              <div>
                <div className="muted" style={{ fontSize: '0.8rem', marginBottom: '4px' }}>Refund Window</div>
                <div style={{ fontWeight: 600, color: '#fff', fontSize: '0.9rem' }}>24h Post-Deadline</div>
              </div>
            </div>
          </section>

          {/* Recent On-Chain Backers */}
          <section className="glass-panel" style={{ padding: '2rem' }}>
            <h2 style={{ fontSize: '1.4rem', color: '#fff', marginBottom: '1.25rem' }}>
              Recent Backers & Transactions
            </h2>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
              {project.recentDonations && project.recentDonations.length > 0 ? (
                project.recentDonations.map((d, i) => (
                  <div
                    key={i}
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      padding: '0.85rem 1rem',
                      borderRadius: '8px',
                      background: 'rgba(255, 255, 255, 0.02)',
                      fontSize: '0.88rem'
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                      <span className="ok-dot" />
                      <span style={{ fontWeight: 600, color: '#fff' }}>{d.donor}</span>
                      <span className="muted" style={{ fontSize: '0.8rem' }}>({d.time})</span>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                      <span style={{ fontWeight: 700, color: 'var(--accent)' }}>
                        +{d.amount.toLocaleString()} XLM
                      </span>
                      <span className="muted" style={{ fontSize: '0.75rem' }}>
                        tx: {d.txHash.slice(0, 8)}...
                      </span>
                    </div>
                  </div>
                ))
              ) : (
                <div className="muted" style={{ padding: '1rem 0', textAlign: 'center' }}>
                  No donations recorded yet. Be the first to back this initiative!
                </div>
              )}
            </div>
          </section>
        </div>

        {/* Right Column: Interactive Funding Escrow Box */}
        <div style={{ position: 'sticky', top: '96px' }}>
          <div className="glass-panel" style={{
            padding: '2.5rem 2rem',
            border: '1px solid rgba(0, 210, 255, 0.3)',
            boxShadow: '0 12px 40px rgba(0, 0, 0, 0.5)'
          }}>
            <div className="pill funding" style={{ marginBottom: '1.25rem', fontSize: '0.75rem' }}>
              Soroban Escrow Pool
            </div>

            <div style={{ display: 'flex', alignItems: 'baseline', gap: '6px', marginBottom: '0.25rem' }}>
              <span style={{ fontSize: '2.8rem', fontWeight: 900, color: 'var(--accent)', letterSpacing: '-0.02em' }}>
                {project.current.toLocaleString()}
              </span>
              <span style={{ fontSize: '1.2rem', fontWeight: 700, color: 'var(--text)' }}>
                {project.token}
              </span>
            </div>

            <div className="muted" style={{ fontSize: '0.9rem', marginBottom: '1.5rem' }}>
              raised of <strong>{project.goal.toLocaleString()} {project.token}</strong> goal ({progress}%)
            </div>

            {/* Progress bar */}
            <div style={{
              width: '100%',
              height: '10px',
              background: 'rgba(255,255,255,0.1)',
              borderRadius: '5px',
              overflow: 'hidden',
              marginBottom: '1.75rem'
            }}>
              <div style={{
                width: `${progress}%`,
                height: '100%',
                background: 'linear-gradient(90deg, var(--primary), var(--accent))',
                borderRadius: '5px',
                transition: 'width 0.8s cubic-bezier(0.4, 0, 0.2, 1)'
              }} />
            </div>

            {/* Quick stats grid */}
            <div style={{
              display: 'grid',
              gridTemplateColumns: '1fr 1fr',
              gap: '1rem',
              padding: '1rem 0',
              borderTop: '1px solid var(--glass-border)',
              borderBottom: '1px solid var(--glass-border)',
              marginBottom: '1.75rem'
            }}>
              <div>
                <div className="muted" style={{ fontSize: '0.8rem' }}>Total Backers</div>
                <div style={{ fontSize: '1.2rem', fontWeight: 800, color: '#fff', marginTop: '2px' }}>
                  {project.donorCount}
                </div>
              </div>

              <div>
                <div className="muted" style={{ fontSize: '0.8rem' }}>Timeline</div>
                <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--ok)', marginTop: '2px' }}>
                  {project.deadline}
                </div>
              </div>
            </div>

            {/* Quick donation selector */}
            <div style={{ marginBottom: '1.5rem' }}>
              <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.6rem', color: 'var(--text)' }}>
                Contribute Amount (XLM)
              </label>

              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '8px', marginBottom: '0.75rem' }}>
                {[50, 100, 500, 1000].map(val => (
                  <button
                    key={val}
                    type="button"
                    onClick={() => handleQuickAdd(val)}
                    className="btn"
                    style={{
                      padding: '0.45rem',
                      fontSize: '0.8rem',
                      fontWeight: 700,
                      borderRadius: '8px',
                      background: Number(donateAmount) === val ? 'var(--primary)' : 'rgba(255, 255, 255, 0.05)',
                      color: '#fff',
                      border: '1px solid var(--glass-border)'
                    }}
                  >
                    +{val}
                  </button>
                ))}
              </div>

              <input
                type="number"
                value={donateAmount}
                onChange={e => setDonateAmount(e.target.value)}
                placeholder="Custom amount"
                style={{
                  width: '100%',
                  padding: '0.75rem 1rem',
                  borderRadius: '10px',
                  border: '1px solid var(--glass-border)',
                  background: 'rgba(0, 0, 0, 0.4)',
                  color: '#fff',
                  fontSize: '1.1rem',
                  fontWeight: 700,
                  outline: 'none'
                }}
              />
            </div>

            {/* Fund CTA button */}
            <button
              onClick={handleInitiateDonation}
              className="btn btn-accent"
              style={{
                width: '100%',
                padding: '1rem',
                fontSize: '1.05rem',
                fontWeight: 800,
                borderRadius: '12px',
                marginBottom: '1.5rem'
              }}
            >
              {wallet.isConnected ? `Back with ${donateAmount || 0} XLM` : 'Connect Wallet to Back'}
            </button>

            {/* Live Ledger Freshness box */}
            <div style={{
              borderRadius: '12px',
              background: 'rgba(0, 0, 0, 0.3)',
              padding: '1rem',
              border: '1px solid var(--glass-border)',
              fontSize: '0.8rem'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '8px', fontWeight: 600, color: 'var(--accent)' }}>
                <span className="ok-dot" />
                <span>Stellar Ledger Synchronized</span>
              </div>
              <LedgerRoutePanel />
            </div>
          </div>
        </div>
      </div>

      {/* Interactive Donation Confirmation Modal */}
      <Modal
        isOpen={isDonateModalOpen}
        onClose={handleCloseModal}
        title={donateStep === 'success' ? '' : 'Confirm Soroban Escrow Contribution'}
        maxWidth="460px"
      >
        {donateStep === 'idle' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <p className="muted" style={{ margin: 0, fontSize: '0.92rem', lineHeight: 1.5 }}>
              You are authorizing a smart contract transfer of <strong>{donateAmount} XLM</strong> to <strong>{project.title}</strong>.
            </p>

            <div className="glass-panel" style={{ padding: '1rem', background: 'rgba(0,0,0,0.3)', fontSize: '0.85rem' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
                <span className="muted">Your Balance:</span>
                <span style={{ fontWeight: 700, color: '#fff' }}>{wallet.balance.toLocaleString()} XLM</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '6px' }}>
                <span className="muted">Contribution:</span>
                <span style={{ fontWeight: 700, color: 'var(--accent)' }}>-{donateAmount} XLM</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid var(--glass-border)', paddingTop: '6px' }}>
                <span className="muted">Balance After:</span>
                <span style={{ fontWeight: 700, color: '#fff' }}>
                  {(wallet.balance - Number(donateAmount)).toLocaleString()} XLM
                </span>
              </div>
            </div>

            <div style={{ display: 'flex', gap: '1rem', marginTop: '0.5rem' }}>
              <button
                type="button"
                className="btn btn-outline"
                style={{ flex: 1, padding: '0.85rem' }}
                onClick={handleCloseModal}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-primary"
                style={{ flex: 2, padding: '0.85rem', fontWeight: 700 }}
                onClick={handleConfirmDonation}
              >
                Sign with {wallet.walletType || 'Wallet'}
              </button>
            </div>
          </div>
        )}

        {donateStep === 'loading' && (
          <div style={{ textAlign: 'center', padding: '2.5rem 1rem', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1.25rem' }}>
            <div style={{
              width: '48px',
              height: '48px',
              borderRadius: '50%',
              border: '4px solid rgba(0, 210, 255, 0.15)',
              borderTopColor: 'var(--accent)',
              animation: 'spin 0.8s linear infinite'
            }} />
            <h3 style={{ margin: 0, fontSize: '1.2rem', color: '#fff' }}>Signing Soroban Invocation</h3>
            <p className="muted" style={{ margin: 0, fontSize: '0.88rem' }}>
              Awaiting signature approval from {wallet.walletType || 'Freighter'} and enrolling transaction in the active ledger block...
            </p>
          </div>
        )}

        {donateStep === 'success' && (
          <div style={{ textAlign: 'center', padding: '1.5rem 0.5rem' }}>
            <div style={{
              width: '64px',
              height: '64px',
              borderRadius: '50%',
              background: 'rgba(16, 185, 129, 0.15)',
              color: 'var(--ok)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '2rem',
              margin: '0 auto 1.25rem'
            }}>
              ✓
            </div>

            <h2 style={{ fontSize: '1.5rem', color: '#fff', marginBottom: '0.5rem' }}>Contribution Confirmed!</h2>
            <p className="muted" style={{ fontSize: '0.92rem', marginBottom: '1.5rem', lineHeight: 1.5 }}>
              Successfully transferred <strong>{donateAmount} XLM</strong> into the PIFP milestone escrow. Funds are locked cryptographically.
            </p>

            <div style={{
              padding: '0.75rem',
              borderRadius: '8px',
              background: 'rgba(0,0,0,0.4)',
              border: '1px solid var(--glass-border)',
              fontSize: '0.8rem',
              color: 'var(--muted)',
              marginBottom: '1.5rem'
            }}>
              Tx Hash: <code style={{ color: 'var(--accent)' }}>{lastTxHash}</code>
            </div>

            <button
              onClick={handleCloseModal}
              className="btn btn-accent"
              style={{ width: '100%', padding: '0.85rem' }}
            >
              Done
            </button>
          </div>
        )}
      </Modal>
    </div>
  )
}
