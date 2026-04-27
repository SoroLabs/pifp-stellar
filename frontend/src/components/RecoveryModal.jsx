export function RecoveryModal({ isOpen, diagnostics, fallbackMessage, onClose }) {
  if (!isOpen) return null

  const steps = diagnostics?.recovery_steps || [
    'Retry once after checking network connectivity.',
    'If this keeps failing, contact support with the transaction hash.',
  ]

  return (
    <div className="recovery-overlay" role="dialog" aria-modal="true" aria-labelledby="recovery-title">
      <div className="recovery-modal">
        <header>
          <p className="recovery-label">Transaction Recovery</p>
          <h2 id="recovery-title">We found a recoverable transaction failure</h2>
        </header>

        <section className="recovery-section">
          <h3>Protocol issue</h3>
          <p>{diagnostics?.protocol_issue || fallbackMessage || 'Transaction failed for an unknown reason.'}</p>
        </section>

        <section className="recovery-section">
          <h3>Suggested next steps</h3>
          <ol>
            {steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        </section>

        {diagnostics?.tx_hash && (
          <section className="recovery-section">
            <h3>Trace data</h3>
            <p>
              <strong>Tx hash:</strong> {diagnostics.tx_hash}
            </p>
            {diagnostics?.soroban_error_code !== null && diagnostics?.soroban_error_code !== undefined && (
              <p>
                <strong>Soroban error code:</strong> {diagnostics.soroban_error_code}
              </p>
            )}
          </section>
        )}

        <footer className="recovery-actions">
          <button type="button" onClick={onClose}>Dismiss</button>
        </footer>
      </div>
    </div>
  )
}
