'use client'

import Link from 'next/link'

export default function Footer() {
  return (
    <footer style={{
      borderTop: '1px solid var(--glass-border)',
      background: 'rgba(3, 7, 15, 0.95)',
      marginTop: 'auto',
      padding: '4rem 0 2.5rem'
    }}>
      <div className="container">
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
          gap: '3rem',
          marginBottom: '3rem'
        }}>
          {/* Col 1 */}
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '1rem' }}>
              <span style={{ fontSize: '1.2rem', color: 'var(--accent)' }}>✦</span>
              <span style={{ fontSize: '1.15rem', fontWeight: 800, color: '#fff' }}>
                PIFP Protocol
              </span>
            </div>
            <p className="muted" style={{ fontSize: '0.88rem', lineHeight: 1.6, margin: '0 0 1rem 0' }}>
              The Public Impact Funding Protocol (PIFP) enables milestone-verified crowdfunding on Stellar Soroban. Trustless escrow with automated oracle consensus.
            </p>
            <div className="pill" style={{ fontSize: '0.75rem' }}>
              <span className="ok-dot" /> Soroban Testnet v25.3.1
            </div>
          </div>

          {/* Col 2 */}
          <div>
            <h4 style={{ color: '#fff', fontSize: '0.95rem', marginBottom: '1.2rem' }}>Ecosystem</h4>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem', fontSize: '0.88rem' }}>
              <Link href="/explore" style={{ color: 'var(--muted)', transition: 'color 0.2s' }}>
                Explore Projects
              </Link>
              <Link href="/#protocol-features" style={{ color: 'var(--muted)', transition: 'color 0.2s' }}>
                How Escrow Works
              </Link>
              <Link href="/#ledger-section" style={{ color: 'var(--muted)', transition: 'color 0.2s' }}>
                Stellar Ledger Gate
              </Link>
              <a href="https://stellar.org" target="_blank" rel="noreferrer" style={{ color: 'var(--muted)', transition: 'color 0.2s' }}>
                Stellar Network ↗
              </a>
            </div>
          </div>

          {/* Col 3 */}
          <div>
            <h4 style={{ color: '#fff', fontSize: '0.95rem', marginBottom: '1.2rem' }}>Smart Contracts</h4>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem', fontSize: '0.88rem' }}>
              <a href="https://soroban.stellar.org/docs" target="_blank" rel="noreferrer" style={{ color: 'var(--muted)' }}>
                Soroban Rust SDK ↗
              </a>
              <span style={{ color: 'var(--muted)', fontSize: '0.82rem' }}>
                Contract: <code style={{ color: 'var(--accent)', background: 'rgba(255,255,255,0.05)', padding: '2px 6px', borderRadius: '4px' }}>CB3...79201</code>
              </span>
              <span style={{ color: 'var(--muted)', fontSize: '0.82rem' }}>
                Oracle Gateway: <code style={{ color: 'var(--ok)', background: 'rgba(255,255,255,0.05)', padding: '2px 6px', borderRadius: '4px' }}>Active (Threshold 2/3)</code>
              </span>
            </div>
          </div>

          {/* Col 4 */}
          <div>
            <h4 style={{ color: '#fff', fontSize: '0.95rem', marginBottom: '1.2rem' }}>Performance Architecture</h4>
            <p className="muted" style={{ fontSize: '0.85rem', lineHeight: 1.5, margin: 0 }}>
              Equipped with predictive client-side pointer prefetching and 5-second ledger-close cache invalidation via custom Service Worker.
            </p>
          </div>
        </div>

        {/* Bottom bar */}
        <div style={{
          borderTop: '1px solid var(--glass-border)',
          paddingTop: '2rem',
          display: 'flex',
          flexWrap: 'wrap',
          justifyContent: 'space-between',
          alignItems: 'center',
          gap: '1rem',
          fontSize: '0.82rem',
          color: 'var(--muted)'
        }}>
          <div>
            © {new Date().getFullYear()} PIFP Stellar. Verified Public Good Protocol.
          </div>
          <div style={{ display: 'flex', gap: '1.5rem' }}>
            <span>Stellar Horizon Testnet</span>
            <span>•</span>
            <span>Soroban RPC 25.3</span>
          </div>
        </div>
      </div>
    </footer>
  )
}
