import React from 'react';
import { usePifpEvents } from '../hooks/usePifpEvents';

const STATUS_LABEL = {
  connecting: '⟳ Connecting…',
  connected: '● Live',
  disconnected: '○ Disconnected',
  error: '✕ Error',
};

const STATUS_CLASS = {
  connecting: 'status-connecting',
  connected: 'status-connected',
  disconnected: 'status-disconnected',
  error: 'status-error',
};

const RealtimeActivity = () => {
  const { events, status, clearEvents } = usePifpEvents({}, 10);

  return (
    <div className="activity-feed">
      <div className="activity-header">
        <h4>Real-time Activity Feed</h4>
        <div className="activity-controls">
          <span className={`status-badge ${STATUS_CLASS[status]}`}>
            {STATUS_LABEL[status]}
          </span>
          {events.length > 0 && (
            <button className="clear-btn" onClick={clearEvents} aria-label="Clear events">
              Clear
            </button>
          )}
        </div>
      </div>

      {events.length === 0 && status === 'connected' && (
        <p className="empty">Waiting for events…</p>
      )}
      {status === 'disconnected' && (
        <p className="empty">Not connected — reconnecting automatically.</p>
      )}

      <ul className="activity-list" aria-live="polite" aria-label="Recent contract events">
        {events.map((activity, i) => (
          <li key={activity.tx_hash ?? i} className="activity-item animate-slide-in">
            <span className="type-badge">
              {activity.event_type.replace(/_/g, ' ')}
            </span>
            {activity.actor && (
              <span className="actor" title={activity.actor}>
                {activity.actor.substring(0, 8)}…
              </span>
            )}
            {activity.amount && (
              <span className="amount">{activity.amount} tokens</span>
            )}
            <span className="ledger">L#{activity.ledger}</span>
          </li>
        ))}
      </ul>

      <style>{`
        .activity-feed {
          background: #1e293b;
          border-radius: 12px;
          padding: 1.5rem;
          border: 1px solid #334155;
          margin-top: 2rem;
        }
        .activity-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-bottom: 0.5rem;
        }
        .activity-header h4 { margin: 0; color: #f1f5f9; }
        .activity-controls { display: flex; align-items: center; gap: 8px; }
        .status-badge {
          font-size: 0.75rem;
          padding: 2px 8px;
          border-radius: 999px;
          font-weight: 600;
        }
        .status-connected { background: #14532d; color: #4ade80; }
        .status-connecting { background: #1e3a5f; color: #60a5fa; }
        .status-disconnected { background: #3f3f46; color: #a1a1aa; }
        .status-error { background: #450a0a; color: #f87171; }
        .clear-btn {
          background: none;
          border: 1px solid #475569;
          color: #94a3b8;
          border-radius: 6px;
          padding: 2px 8px;
          font-size: 0.75rem;
          cursor: pointer;
        }
        .clear-btn:hover { border-color: #94a3b8; color: #f1f5f9; }
        .activity-list { list-style: none; padding: 0; margin: 1rem 0 0 0; }
        .activity-item {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 10px;
          border-bottom: 1px solid #334155;
          font-size: 0.9rem;
          color: #f1f5f9;
        }
        .activity-item:last-child { border-bottom: none; }
        .type-badge {
          background: #1d4ed8;
          color: #bfdbfe;
          border-radius: 6px;
          padding: 2px 8px;
          font-size: 0.75rem;
          text-transform: capitalize;
          white-space: nowrap;
        }
        .actor { color: #94a3b8; font-family: monospace; font-size: 0.8rem; }
        .amount { color: #4ade80; font-weight: 600; }
        .ledger { color: #64748b; font-size: 0.8rem; margin-left: auto; }
        .empty { color: #64748b; font-size: 0.9rem; margin-top: 1rem; }
        @keyframes slideIn {
          from { opacity: 0; transform: translateY(-6px); }
          to   { opacity: 1; transform: translateY(0); }
        }
        .animate-slide-in { animation: slideIn 0.2s ease-out; }
      `}</style>
    </div>
  );
};

export default RealtimeActivity;
