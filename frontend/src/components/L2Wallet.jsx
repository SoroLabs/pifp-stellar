import { useMemo, useState } from 'react'

const ORACLE_API = (import.meta.env.VITE_ORACLE_API_URL || 'http://localhost:9090/api').replace(/\/$/, '')
const DEFAULT_CHAIN = import.meta.env.VITE_ROLLUP_CHAIN_ID || 'soroban-testnet'
const DEFAULT_DOMAIN = 'pifp-rollup-v1'

function encodeBase64(bytes) {
  let binary = ''
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i])
  }
  return window.btoa(binary)
}

function canonicalIntentMessage({ donor, projectId, amountStroops, nonce, expiresAt }) {
  return [
    'PIFP_ROLLUP_INTENT',
    `domain:${DEFAULT_DOMAIN}`,
    `chain_id:${DEFAULT_CHAIN}`,
    `donor:${donor}`,
    `project_id:${projectId}`,
    `amount_stroops:${amountStroops}`,
    `nonce:${nonce}`,
    `expires_at:${expiresAt}`,
  ].join('\n')
}

export function L2Wallet() {
  const [l2Mode, setL2Mode] = useState(true)
  const [donor, setDonor] = useState('G-L2-EXAMPLE-ADDRESS')
  const [projectId, setProjectId] = useState('1')
  const [amount, setAmount] = useState('0.01')
  const [nonce, setNonce] = useState('1')
  const [expiresInSeconds, setExpiresInSeconds] = useState('300')
  const [wallet, setWallet] = useState(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [lastIntent, setLastIntent] = useState('')
  const [balance, setBalance] = useState({ pending_stroops: 0, confirmed_stroops: 0 })

  const amountStroops = useMemo(() => {
    const parsed = Number.parseFloat(amount)
    if (!Number.isFinite(parsed) || parsed <= 0) return 0
    return Math.floor(parsed * 10_000_000)
  }, [amount])

  async function ensureWallet() {
    if (wallet) return wallet

    if (!window.crypto?.subtle) {
      throw new Error('WebCrypto is required for L2 signing in this browser')
    }

    const keyPair = await window.crypto.subtle.generateKey(
      {
        name: 'Ed25519',
      },
      true,
      ['sign', 'verify'],
    )

    const rawPub = await window.crypto.subtle.exportKey('raw', keyPair.publicKey)
    const exportedPubKey = encodeBase64(new Uint8Array(rawPub))
    const newWallet = {
      publicKey: exportedPubKey,
      sign: async (messageText) => {
        const signature = await window.crypto.subtle.sign(
          { name: 'Ed25519' },
          keyPair.privateKey,
          new TextEncoder().encode(messageText),
        )
        return encodeBase64(new Uint8Array(signature))
      },
    }

    setWallet(newWallet)
    return newWallet
  }

  async function refreshBalance(address) {
    const response = await fetch(`${ORACLE_API}/rollup/balance/${encodeURIComponent(address)}`)
    if (!response.ok) {
      throw new Error(`Failed to fetch L2 balance (${response.status})`)
    }
    const payload = await response.json()
    setBalance(payload)
  }

  async function submitL2Intent() {
    setError('')
    setBusy(true)

    try {
      if (!l2Mode) {
        throw new Error('Enable L2 Wallet mode to submit off-chain intent')
      }
      if (!donor.trim()) {
        throw new Error('Donor address is required')
      }
      if (!Number.isInteger(Number(projectId)) || Number(projectId) <= 0) {
        throw new Error('Project ID must be a positive integer')
      }
      if (amountStroops <= 0) {
        throw new Error('Amount must be greater than zero')
      }

      const walletHandle = await ensureWallet()
      const now = Math.floor(Date.now() / 1000)
      const expiresAt = now + Math.max(30, Number.parseInt(expiresInSeconds, 10) || 300)

      const message = canonicalIntentMessage({
        donor: donor.trim(),
        projectId: Number(projectId),
        amountStroops,
        nonce: Number(nonce),
        expiresAt,
      })

      const signature = await walletHandle.sign(message)

      const response = await fetch(`${ORACLE_API}/rollup/intents`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          domain: DEFAULT_DOMAIN,
          chain_id: DEFAULT_CHAIN,
          donor: donor.trim(),
          project_id: Number(projectId),
          amount_stroops: amountStroops,
          nonce: Number(nonce),
          expires_at: expiresAt,
          message,
          signature,
          public_key: walletHandle.publicKey,
        }),
      })

      const payload = await response.json()
      if (!response.ok) {
        throw new Error(payload.error || `Oracle returned ${response.status}`)
      }

      setLastIntent(payload.intent_id)
      await refreshBalance(donor.trim())
      setNonce((current) => String(Number(current) + 1))
    } catch (err) {
      setError(err.message || 'Failed to submit L2 intent')
    } finally {
      setBusy(false)
    }
  }

  async function settleBatch() {
    setBusy(true)
    setError('')
    try {
      const response = await fetch(`${ORACLE_API}/rollup/settle`, { method: 'POST' })
      if (!response.ok) {
        throw new Error(`Settlement failed (${response.status})`)
      }
      await refreshBalance(donor.trim())
    } catch (err) {
      setError(err.message || 'Failed to settle rollup batch')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="l2-wallet" aria-live="polite">
      <header className="page-header">
        <h2>L2 Wallet Micro-Rollup</h2>
        <p>
          Sign off-chain authorization messages for micro-donations, queue them in the Rust sequencer,
          and settle them in periodic Soroban batches.
        </p>
      </header>

      <div className="l2-grid">
        <div className="card">
          <label className="toggle-row">
            <input
              type="checkbox"
              checked={l2Mode}
              onChange={(e) => setL2Mode(e.target.checked)}
            />
            <span>Enable L2 Wallet mode (off-chain signatures)</span>
          </label>

          <label>
            <span>Donor Address</span>
            <input value={donor} onChange={(e) => setDonor(e.target.value)} />
          </label>

          <div className="inline-grid">
            <label>
              <span>Project ID</span>
              <input value={projectId} onChange={(e) => setProjectId(e.target.value)} />
            </label>
            <label>
              <span>Amount (XLM)</span>
              <input value={amount} onChange={(e) => setAmount(e.target.value)} />
            </label>
          </div>

          <div className="inline-grid">
            <label>
              <span>Nonce</span>
              <input value={nonce} onChange={(e) => setNonce(e.target.value)} />
            </label>
            <label>
              <span>TTL (seconds)</span>
              <input value={expiresInSeconds} onChange={(e) => setExpiresInSeconds(e.target.value)} />
            </label>
          </div>

          <div className="actions-row">
            <button type="button" onClick={submitL2Intent} disabled={busy}>
              {busy ? 'Submitting...' : 'Sign Off-Chain Authorization'}
            </button>
            <button type="button" onClick={settleBatch} disabled={busy} className="ghost-btn">
              Settle Current Batch
            </button>
          </div>

          {error && <p className="error">{error}</p>}
          {lastIntent && (
            <p className="meta">
              Last intent ID: <code>{lastIntent.slice(0, 20)}...</code>
            </p>
          )}
        </div>

        <div className="card balances-card">
          <h3>Rollup Balance View</h3>
          <p className="meta">Comparing off-chain pending and on-chain-confirmed totals.</p>
          <div className="balance-row">
            <span>Pending (L2 queue)</span>
            <strong>{balance.pending_stroops} stroops</strong>
          </div>
          <div className="balance-row">
            <span>Confirmed (on-chain)</span>
            <strong>{balance.confirmed_stroops} stroops</strong>
          </div>

          <div className="typed-preview">
            <h4>Typed Data Preview</h4>
            <pre>
              {canonicalIntentMessage({
                donor: donor.trim() || '<donor>',
                projectId: Number(projectId) || 0,
                amountStroops,
                nonce: Number(nonce) || 0,
                expiresAt: Math.floor(Date.now() / 1000) + (Number(expiresInSeconds) || 300),
              })}
            </pre>
          </div>

          {wallet?.publicKey && (
            <p className="meta">Session public key: <code>{wallet.publicKey.slice(0, 24)}...</code></p>
          )}
        </div>
      </div>
    </section>
  )
}
