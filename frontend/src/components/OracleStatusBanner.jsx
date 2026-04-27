/**
 * OracleStatusBanner – fetches /oracle/quote and renders a health indicator.
 *
 * Green  (health_score >= 80): all clear, protocol actions enabled.
 * Yellow (health_score 55-79): degraded, advisory warning shown.
 * Red    (health_score  < 55): critical / stale / high-variance; disable critical actions + modal.
 */

import { useCallback, useEffect, useRef, useState } from 'react'

const ORACLE_BASE = (
  import.meta.env.VITE_ORACLE_API_URL || 'http://localhost:9090'
).replace(/\/$/, '')

const POLL_INTERVAL_MS = 15_000

/**
 * React hook that polls the oracle /oracle/quote endpoint.
 * @returns {{ oracleStatus: object|null, oracleBlocking: boolean, refreshOracle: function }}
 */
export function useOracleStatus() {
  const [oracleStatus, setOracleStatus] = useState(null)
  const abortRef = useRef(null)

  const fetchStatus = useCallback(async () => {
    if (abortRef.current) abortRef.current.abort()
    const controller = new AbortController()
    abortRef.current = controller

    try {
      const res = await fetch(`${ORACLE_BASE}/oracle/quote`, {
        signal: controller.signal,
      })
      if (!res.ok) throw new Error(`Oracle API returned ${res.status}`)
      const data = await res.json()
      setOracleStatus(data)
    } catch (err) {
      if (err.name !== 'AbortError') {
        setOracleStatus((prev) =>
          prev
            ? { ...prev, _fetchError: err.message }
            : {
                indicator: 'red',
                status: 'critical',
                health_score: 0,
                stale: true,
                high_variance: false,
                summary: 'Oracle service unreachable.',
                reasons: [err.message],
                recovery_actions: [],
                providers: [],
              }
        )
      }
    }
  }, [])

  useEffect(() => {
    fetchStatus()
    const timer = setInterval(fetchStatus, POLL_INTERVAL_MS)
    return () => {
      clearInterval(timer)
      if (abortRef.current) abortRef.current.abort()
    }
  }, [fetchStatus])

  const oracleBlocking =
    oracleStatus !== null &&
    (oracleStatus.stale || oracleStatus.high_variance || oracleStatus.indicator === 'red')

  return { oracleStatus, oracleBlocking, refreshOracle: fetchStatus }
}

/**
 * Renders the oracle status pill, price info, and (when degraded/critical) a details modal.
 *
 * @param {{ showModal: boolean, onModalClose: function }} props
 */
export function OracleStatusBanner({ showModal, onModalClose }) {
  const { oracleStatus, oracleBlocking, refreshOracle } = useOracleStatus()

  if (!oracleStatus) {
    return (
      <div className="oracle-banner oracle-banner--loading" role="status" aria-live="polite">
        <span className="oracle-pill oracle-pill--loading" aria-label="Oracle loading">⏳</span>
        <span>Checking oracle feed…</span>
      </div>
    )
  }

  const {
    indicator,
    status,
    health_score,
    aggregated_price,
    quote_symbol,
    asset_symbol,
    variance_pct,
    max_sample_age_secs,
    summary,
    reasons,
    recovery_actions,
    providers,
  } = oracleStatus

  const pillLabel = { green: '● Healthy', yellow: '● Degraded', red: '● Critical' }[indicator] ?? '● Unknown'

  return (
    <>
      <div
        className={`oracle-banner oracle-banner--${indicator}`}
        role="status"
        aria-live="polite"
        aria-label={`Oracle status: ${status}`}
      >
        <span
          className={`oracle-pill oracle-pill--${indicator}`}
          title={`Health score: ${health_score}/100`}
        >
          {pillLabel}
        </span>

        {aggregated_price != null ? (
          <span className="oracle-price">
            {asset_symbol}/{quote_symbol}: <strong>${aggregated_price}</strong>
          </span>
        ) : (
          <span className="oracle-price oracle-price--unavailable">Price unavailable</span>
        )}

        <span className="oracle-meta">
          Age: {max_sample_age_secs}s · Variance: {variance_pct}%
        </span>

        {indicator !== 'green' && (
          <button
            className="oracle-detail-btn"
            onClick={onModalClose}
            aria-expanded={showModal}
            aria-controls="oracle-detail-modal"
          >
            {showModal ? 'Hide details' : 'View details'}
          </button>
        )}

        <button
          className="oracle-refresh-btn"
          onClick={refreshOracle}
          aria-label="Refresh oracle feed"
          title="Refresh now"
        >
          ↻
        </button>
      </div>

      {showModal && indicator !== 'green' && (
        <div
          id="oracle-detail-modal"
          className={`oracle-modal oracle-modal--${indicator}`}
          role="dialog"
          aria-modal="true"
          aria-label="Oracle status details"
        >
          <div className="oracle-modal__backdrop" onClick={onModalClose} aria-hidden="true" />
          <div className="oracle-modal__content">
            <button
              className="oracle-modal__close"
              onClick={onModalClose}
              aria-label="Close oracle details"
            >
              ✕
            </button>

            <h2 className="oracle-modal__title">
              <span className={`oracle-pill oracle-pill--${indicator}`}>{pillLabel}</span>
              Oracle {status.charAt(0).toUpperCase() + status.slice(1)}
            </h2>

            <p className="oracle-modal__summary">{summary}</p>

            {oracleBlocking && (
              <div className="oracle-modal__block-notice" role="alert">
                <strong>Protocol actions are paused</strong> while the oracle is in this state.
                Critical actions will remain disabled until the indicator returns to green.
              </div>
            )}

            {reasons.length > 0 && (
              <section className="oracle-modal__section">
                <h3>Reasons</h3>
                <ul>
                  {reasons.map((r, i) => (
                    <li key={i}>{r}</li>
                  ))}
                </ul>
              </section>
            )}

            {recovery_actions.length > 0 && (
              <section className="oracle-modal__section">
                <h3>Recovery actions</h3>
                <ol>
                  {recovery_actions.map((a, i) => (
                    <li key={i}>{a}</li>
                  ))}
                </ol>
              </section>
            )}

            {providers.length > 0 && (
              <section className="oracle-modal__section oracle-modal__providers">
                <h3>Provider observations</h3>
                <table aria-label="Provider observations">
                  <thead>
                    <tr>
                      <th>Provider</th>
                      <th>Price</th>
                      <th>Age (s)</th>
                      <th>Status</th>
                      <th>Detail</th>
                    </tr>
                  </thead>
                  <tbody>
                    {providers.map((p) => (
                      <tr key={p.provider}>
                        <td>{p.provider}</td>
                        <td>{p.price != null ? `$${p.price}` : '—'}</td>
                        <td>{p.age_secs}</td>
                        <td>
                          <span className={`oracle-provider-badge oracle-provider-badge--${p.status}`}>
                            {p.status}
                          </span>
                        </td>
                        <td className="oracle-provider-detail">{p.detail}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </section>
            )}

            <div className="oracle-modal__footer">
              <button className="oracle-modal__close-btn" onClick={onModalClose}>
                Close
              </button>
              <button className="oracle-refresh-btn" onClick={refreshOracle}>
                ↻ Refresh now
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
