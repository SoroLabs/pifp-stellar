'use client'

import Link from 'next/link'
import ProjectCard from '@/components/ProjectCard'
import LedgerRoutePanel from '@/components/LedgerRoutePanel'
import { useApp } from '@/context/AppContext'

export default function HomePage() {
  const { projects, setCreateModalOpen } = useApp()

  const featured = projects.slice(0, 3)

  // Calculate live protocol aggregates
  const totalRaised = projects.reduce((acc, p) => acc + p.current, 0)
  const totalDonors = projects.reduce((acc, p) => acc + p.donorCount, 0)

  return (
    <div className="container animate-fade-in" style={{ padding: '2rem 1.5rem 5rem' }}>
      {/* Hero Section */}
      <section style={{
        textAlign: 'center',
        padding: '5rem 0 4rem',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        position: 'relative'
      }}>
        {/* Background glow orb */}
        <div style={{
          position: 'absolute',
          top: '20%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: '500px',
          height: '300px',
          background: 'radial-gradient(ellipse, rgba(0, 210, 255, 0.15), transparent 70%)',
          filter: 'blur(50px)',
          pointerEvents: 'none',
          zIndex: 0
        }} />

        <div style={{ position: 'relative', zIndex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
          <div className="pill funding" style={{ marginBottom: '1.5rem', padding: '0.35rem 0.9rem' }}>
            <span className="ok-dot" /> Soroban Smart Contracts V25.3.1 Active
          </div>

          <h1 style={{
            fontSize: 'clamp(2.5rem, 5.5vw, 4.5rem)',
            maxWidth: '900px',
            margin: '0 0 1.5rem 0',
            letterSpacing: '-0.03em',
            fontWeight: 900,
            lineHeight: 1.15
          }}>
            Fund Global Impact with <br />
            <span className="text-accent-gradient">Cryptographic Proof</span>
          </h1>

          <p className="muted" style={{
            fontSize: 'clamp(1.05rem, 2vw, 1.25rem)',
            maxWidth: '680px',
            margin: '0 0 2.5rem 0',
            lineHeight: 1.6
          }}>
            The Public Impact Funding Protocol (PIFP) empowers creators to raise capital transparently on Stellar. Funds are held in trustless escrows and unlocked only when milestones pass decentralized oracle verification.
          </p>

          <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', justifyContent: 'center' }}>
            <Link
              href="/explore"
              data-predictive="true"
              className="btn btn-accent"
              style={{ fontSize: '1.05rem', padding: '0.85rem 2.2rem' }}
            >
              Explore Projects
            </Link>

            <button
              onClick={() => setCreateModalOpen(true)}
              className="btn btn-outline"
              style={{ fontSize: '1.05rem', padding: '0.85rem 2.2rem', gap: '8px' }}
            >
              <span>+</span>
              <span>Launch a Project</span>
            </button>
          </div>
        </div>
      </section>

      {/* Protocol Live Statistics Strip */}
      <section style={{ marginBottom: '5rem' }}>
        <div className="glass-panel" style={{ padding: '2.5rem 2rem', background: 'rgba(10, 18, 36, 0.65)' }}>
          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
            gap: '2rem',
            textAlign: 'center'
          }}>
            <div>
              <div style={{ fontSize: '2.8rem', fontWeight: 900, color: 'var(--accent)' }}>
                {totalRaised.toLocaleString()} <span style={{ fontSize: '1.2rem', color: 'var(--text)' }}>XLM</span>
              </div>
              <div className="muted" style={{ fontWeight: 600, fontSize: '0.9rem', marginTop: '4px' }}>
                Total Capital Escrowed
              </div>
            </div>

            <div>
              <div style={{ fontSize: '2.8rem', fontWeight: 900 }} className="text-gradient">
                {projects.length}
              </div>
              <div className="muted" style={{ fontWeight: 600, fontSize: '0.9rem', marginTop: '4px' }}>
                Active Protocols
              </div>
            </div>

            <div>
              <div style={{ fontSize: '2.8rem', fontWeight: 900 }} className="text-gradient">
                {totalDonors.toLocaleString()}
              </div>
              <div className="muted" style={{ fontWeight: 600, fontSize: '0.9rem', marginTop: '4px' }}>
                On-Chain Backers
              </div>
            </div>

            <div>
              <div style={{ fontSize: '2.8rem', fontWeight: 900, color: 'var(--ok)' }}>
                100%
              </div>
              <div className="muted" style={{ fontWeight: 600, fontSize: '0.9rem', marginTop: '4px' }}>
                Verified Proof Transparency
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Featured Projects Section */}
      <section style={{ marginBottom: '6rem' }}>
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-end',
          marginBottom: '2.5rem',
          flexWrap: 'wrap',
          gap: '1rem'
        }}>
          <div>
            <h2 style={{ fontSize: '2.2rem', marginBottom: '0.4rem', color: '#fff' }}>
              Featured Initiatives
            </h2>
            <p className="muted" style={{ margin: 0, fontSize: '1rem' }}>
              Discover high-impact public good initiatives currently backed by Soroban escrows.
            </p>
          </div>

          <Link
            href="/explore"
            data-predictive="true"
            style={{
              color: 'var(--accent)',
              fontWeight: 600,
              fontSize: '0.95rem',
              display: 'flex',
              alignItems: 'center',
              gap: '6px'
            }}
          >
            View all projects ({projects.length}) →
          </Link>
        </div>

        <div className="grid grid-cols-3">
          {featured.map(project => (
            <ProjectCard key={project.id} {...project} />
          ))}
        </div>
      </section>

      {/* How PIFP Protocol Works */}
      <section id="protocol-features" style={{ marginBottom: '6rem' }}>
        <div style={{ textAlign: 'center', marginBottom: '3.5rem' }}>
          <div className="pill funding" style={{ marginBottom: '1rem' }}>
            Built on Soroban
          </div>
          <h2 style={{ fontSize: '2.4rem', marginBottom: '0.8rem', color: '#fff' }}>
            Trustless Architecture by Design
          </h2>
          <p className="muted" style={{ maxWidth: '600px', margin: '0 auto', fontSize: '1.05rem' }}>
            Traditional crowdfunding relies on blind trust. PIFP leverages Stellar smart contracts to align incentives with verifiable milestones.
          </p>
        </div>

        <div className="grid grid-cols-3">
          <div className="glass-panel" style={{ padding: '2.5rem 2rem' }}>
            <div style={{
              width: '48px',
              height: '48px',
              borderRadius: '12px',
              background: 'rgba(0, 210, 255, 0.15)',
              color: 'var(--accent)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '1.5rem',
              marginBottom: '1.5rem'
            }}>
              🔒
            </div>
            <h3 style={{ fontSize: '1.25rem', marginBottom: '0.75rem', color: '#fff' }}>
              Milestone-Based Escrow
            </h3>
            <p className="muted" style={{ fontSize: '0.92rem', lineHeight: 1.6, margin: 0 }}>
              Donations are locked in a dedicated Soroban contract. Capital releases in predetermined basis-point tranches only as physical work is completed.
            </p>
          </div>

          <div className="glass-panel" style={{ padding: '2.5rem 2rem' }}>
            <div style={{
              width: '48px',
              height: '48px',
              borderRadius: '12px',
              background: 'rgba(58, 130, 246, 0.15)',
              color: 'var(--primary)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '1.5rem',
              marginBottom: '1.5rem'
            }}>
              ⚡
            </div>
            <h3 style={{ fontSize: '1.25rem', marginBottom: '0.75rem', color: '#fff' }}>
              Multi-Oracle Consensus
            </h3>
            <p className="muted" style={{ fontSize: '0.92rem', lineHeight: 1.6, margin: 0 }}>
              Independent environmental and technical oracles must cryptographically sign proof hashes before funds can transition from escrow to the creator.
            </p>
          </div>

          <div className="glass-panel" style={{ padding: '2.5rem 2rem' }}>
            <div style={{
              width: '48px',
              height: '48px',
              borderRadius: '12px',
              background: 'rgba(16, 185, 129, 0.15)',
              color: 'var(--ok)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '1.5rem',
              marginBottom: '1.5rem'
            }}>
              🛡️
            </div>
            <h3 style={{ fontSize: '1.25rem', marginBottom: '0.75rem', color: '#fff' }}>
              Automatic Donor Protection
            </h3>
            <p className="muted" style={{ fontSize: '0.92rem', lineHeight: 1.6, margin: 0 }}>
              If a project fails to reach its funding target or misses milestone verification deadlines, donors can trigger trustless automated refunds.
            </p>
          </div>
        </div>
      </section>

      {/* Stellar Ledger Freshness & Predictive Navigation Gate */}
      <section id="ledger-section" style={{ marginBottom: '3rem' }}>
        <div className="glass-panel" style={{
          padding: '2.5rem',
          border: '1px solid rgba(0, 210, 255, 0.25)',
          background: 'linear-gradient(135deg, rgba(14, 23, 44, 0.85) 0%, rgba(9, 16, 29, 0.95) 100%)'
        }}>
          <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'flex-start',
            flexWrap: 'wrap',
            gap: '1.5rem',
            marginBottom: '1.5rem'
          }}>
            <div>
              <div className="pill" style={{ marginBottom: '0.5rem' }}>
                <span className="ok-dot" /> Real-time Node Sync
              </div>
              <h3 style={{ fontSize: '1.6rem', margin: '0 0 0.5rem 0', color: '#fff' }}>
                Predictive Navigation & Ledger Freshness Gate
              </h3>
              <p className="muted" style={{ margin: 0, maxWidth: '650px', fontSize: '0.95rem' }}>
                This Next.js App Router application dynamically predicts user trajectory, triggers millisecond-early route prefetches via Service Worker, and automatically invalidates stale caches across Stellar 5-second ledger intervals.
              </p>
            </div>

            <div style={{
              padding: '1rem 1.5rem',
              borderRadius: '12px',
              background: 'rgba(0,0,0,0.4)',
              border: '1px solid var(--glass-border)',
              minWidth: '240px'
            }}>
              <div style={{ fontSize: '0.8rem', color: 'var(--muted)', marginBottom: '4px' }}>
                Stellar Network Pulse
              </div>
              <LedgerRoutePanel />
            </div>
          </div>
        </div>
      </section>
    </div>
  )
}
