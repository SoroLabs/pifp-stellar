import React, { useState, useEffect } from 'react';
import { useSubscription } from '@apollo/client';
import { ACTIVITY_SUBSCRIPTION } from '../graphql/queries';

const RealtimeActivity = () => {
  const [activities, setActivities] = useState([]);
  const { data, loading, error } = useSubscription(ACTIVITY_SUBSCRIPTION);

  useEffect(() => {
    if (data?.activityFeed) {
      setActivities(prev => [data.activityFeed, ...prev].slice(0, 10));
    }
  }, [data]);

  return (
    <div className="activity-feed">
      <h4>Real-time Activity Feed</h4>
      {error && <p className="error">Subscription Error: {error.message}</p>}
      {activities.length === 0 && !loading && <p className="empty">Waiting for events...</p>}
      <ul className="activity-list">
        {activities.map((activity, i) => (
          <li key={activity.id || i} className="activity-item animate-slide-in">
            <span className="type-badge">{activity.eventType.replace('_', ' ')}</span>
            <span className="actor">{activity.actor?.substring(0, 8)}...</span>
            {activity.amount && <span className="amount">{activity.amount} tokens</span>}
            <span className="ledger">L#{activity.ledger}</span>
          </li>
        ))}
      </ul>
      <style jsx>{`
        .activity-feed {
          background: #1e293b;
          border-radius: 12px;
          padding: 1.5rem;
          border: 1px solid #334155;
          margin-top: 2rem;
        }
        .activity-list {
          list-style: none;
          padding: 0;
          margin: 1rem 0 0 0;
        }
        .activity-item {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 10px;
          border-bottom: 1px solid #334155;
          font-size: 0.9rem;
          color: #f1f5f9;
        }
        .animate-slide-in {
          animation: slideIn 0.3s ease-out;
        }
        @keyframes slideIn {
          from { opacity: 0; transform: translateY(-10px); }
          to { opacity: 1; transform: translateY(0); }
        }
        .type-badge {
          background: #3b82f6;
          padding: 2px 8px;
          border-radius: 4px;
          text-transform: capitalize;
          font-size: 0.75rem;
        }
        .actor { color: #94a3b8; }
        .amount { font-weight: bold; color: #10b981; }
        .ledger { margin-left: auto; color: #64748b; font-size: 0.8rem; }
      `}</style>
    </div>
  );
};

export default RealtimeActivity;
