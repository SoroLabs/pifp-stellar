import React from 'react';

const VerifiedBadge = ({ isVerified, stateRoot, ledgerSeq }) => {
  if (isVerified === null) return null;

  return (
    <div className={`verified-badge ${isVerified ? 'verified' : 'unverified'}`}>
      <span className="icon">{isVerified ? '✓' : '✗'}</span>
      <span className="text">{isVerified ? 'State Verified' : 'Verification Failed'}</span>
      {isVerified && (
        <div className="tooltip">
          <p>Verified against Stellar Ledger #{ledgerSeq}</p>
          <p className="root">Root: {stateRoot.substring(0, 16)}...</p>
        </div>
      )}
      <style jsx>{`
        .verified-badge {
          display: inline-flex;
          align-items: center;
          padding: 4px 12px;
          border-radius: 20px;
          font-size: 0.85rem;
          font-weight: 600;
          cursor: help;
          position: relative;
          transition: all 0.3s ease;
        }
        .verified {
          background: rgba(16, 185, 129, 0.1);
          color: #10b981;
          border: 1px solid rgba(16, 185, 129, 0.2);
        }
        .unverified {
          background: rgba(239, 68, 68, 0.1);
          color: #ef4444;
          border: 1px solid rgba(239, 68, 68, 0.2);
        }
        .icon {
          margin-right: 6px;
          font-size: 1rem;
        }
        .tooltip {
          visibility: hidden;
          background-color: #1e293b;
          color: #fff;
          text-align: left;
          border-radius: 8px;
          padding: 8px 12px;
          position: absolute;
          z-index: 10;
          bottom: 125%;
          left: 50%;
          transform: translateX(-50%);
          width: 240px;
          opacity: 0;
          transition: opacity 0.3s;
          box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
          font-size: 0.75rem;
        }
        .tooltip p {
          margin: 0;
          line-height: 1.4;
        }
        .tooltip .root {
          font-family: monospace;
          color: #94a3b8;
          margin-top: 4px;
        }
        .verified-badge:hover .tooltip {
          visibility: visible;
          opacity: 1;
        }
      `}</style>
    </div>
  );
};

export default VerifiedBadge;
