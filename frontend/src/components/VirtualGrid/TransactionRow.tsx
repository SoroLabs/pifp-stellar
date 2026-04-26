import React, { useState, useRef, useEffect } from 'react';
import { Transaction } from './useInfiniteFetch';

interface TransactionRowProps {
  item: Transaction | null; // null represents skeleton
  index: number;
  style: React.CSSProperties;
  onMeasure: (index: number, height: number) => void;
}

export function TransactionRow({ item, index, style, onMeasure }: TransactionRowProps) {
  const [expanded, setExpanded] = useState(false);
  const rowRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (rowRef.current) {
      // Use ResizeObserver to detect natural height changes
      const observer = new ResizeObserver((entries) => {
        for (let entry of entries) {
          const rect = entry.target.getBoundingClientRect();
          onMeasure(index, rect.height);
        }
      });
      observer.observe(rowRef.current);

      return () => {
        observer.disconnect();
      };
    }
  }, [index, onMeasure, expanded]);

  if (!item) {
    // Skeleton loader
    return (
      <div style={style} className="transaction-row skeleton" ref={rowRef}>
        <div className="skeleton-line" style={{ width: '40%' }}></div>
        <div className="skeleton-line" style={{ width: '20%' }}></div>
        <div className="skeleton-line" style={{ width: '30%' }}></div>
      </div>
    );
  }

  return (
    <div style={style} className={`transaction-row ${expanded ? 'expanded' : ''}`} ref={rowRef}>
      <div className="transaction-summary" onClick={() => setExpanded(!expanded)}>
        <div className="col hash">{item.hash}</div>
        <div className="col ledger">Ledger: {item.ledger}</div>
        <div className="col fee">{item.fee} XLM</div>
        <div className={`col status ${item.status}`}>{item.status.toUpperCase()}</div>
        <div className="col toggle">{expanded ? '▼' : '▶'}</div>
      </div>
      
      {expanded && (
        <div className="transaction-details">
          <h4>Complex Transaction Payload</h4>
          <pre>
{`{
  "source_account": "GABC...123",
  "fee_account": "GABC...123",
  "operations": [
    {
      "type": "payment",
      "asset": "native",
      "amount": "100.00"
    }
  ],
  "signatures": ["..."]
}`}
          </pre>
        </div>
      )}
    </div>
  );
}
