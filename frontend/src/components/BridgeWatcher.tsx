import { useEffect, useState } from 'react'

const ORACLE_API = (import.meta.env.VITE_ORACLE_API_URL || 'http://localhost:9090/api').replace(/\/$/, '')

export function BridgeWatcher() {
  const [messages, setMessages] = useState([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    async function loadMessages() {
      try {
        const response = await fetch(`${ORACLE_API}/bridge/messages`)
        if (!response.ok) throw new Error(`Oracle returned ${response.status}`)
        const data = await response.json()
        setMessages(data)
      } catch (err) {
        setError(err.message)
      } finally {
        setIsLoading(false)
      }
    }

    loadMessages()
    const interval = setInterval(loadMessages, 5000)
    return () => clearInterval(interval)
  }, [])

  const handleSign = async (id) => {
    try {
      await fetch(`${ORACLE_API}/bridge/sign/${id}`, { method: 'POST' })
    } catch (err) {
      console.error('Failed to sign:', err)
    }
  }

  return (
    <div className="bridge-watcher">
      <header className="page-header">
        <h2>Cross-Chain Bridge Observer</h2>
        <p>Real-time monitoring of foreign chain events and validator signatures.</p>
      </header>

      {isLoading && <p>Loading bridge state...</p>}
      {error && <p className="error">Error: {error}</p>}

      <div className="message-list">
        {!isLoading && messages.map((msg) => (
          <div key={msg.id} className="message-card">
            <div className="card-header">
              <span className="tx-id">{msg.id}</span>
              <span className={`status-pill ${msg.status.toLowerCase()}`}>{msg.status}</span>
            </div>
            <div className="card-body">
              <div className="route">
                <strong>{msg.source_chain}</strong>
                <span className="arrow">→</span>
                <strong>{msg.target_chain}</strong>
              </div>
              <div className="details">
                <p><span>Amount:</span> {msg.amount}</p>
                <p><span>Recipient:</span> {msg.recipient}</p>
              </div>
              <div className="signature-progress">
                <div className="progress-bar">
                  <div 
                    className="progress-fill" 
                    style={{ width: `${(msg.signatures_collected / msg.total_required) * 100}%` }}
                  ></div>
                </div>
                <div className="progress-text">
                  {msg.signatures_collected} / {msg.total_required} signatures collected
                </div>
              </div>
            </div>
            <div className="card-actions">
              <button 
                onClick={() => handleSign(msg.id)}
                disabled={msg.signatures_collected >= msg.total_required}
              >
                {msg.signatures_collected >= msg.total_required ? 'Fully Signed' : 'Add My Signature'}
              </button>
            </div>
          </div>
        ))}
        {!isLoading && messages.length === 0 && <p>No active cross-chain messages.</p>}
      </div>
    </div>
  )
}
