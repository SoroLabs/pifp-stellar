import { useEffect, useRef, useState } from 'react';
import { createMachine } from 'xstate';
import { useMachine } from '@xstate/react';
import { motion } from 'framer-motion';
import { usePifpEvents } from '../hooks/usePifpEvents';
import { useTransactions } from '../context/TransactionContext';

/**
 * Trade state machine.
 *
 * idle → optimistic (balance deducted immediately)
 *   → confirmed  (funds_released or project_funded event received for this tx)
 *   → rolledBack (project_expired / project_cancelled received, or timeout)
 */
const tradeMachine = createMachine({
  id: 'trade',
  initial: 'idle',
  states: {
    idle: { on: { SUBMIT: 'optimistic' } },
    optimistic: { on: { CONFIRM: 'confirmed', REJECT: 'rolledBack' } },
    confirmed: { type: 'final' },
    rolledBack: { type: 'final' },
  },
});

/** How long to wait for an on-chain confirmation before rolling back (ms). */
const CONFIRMATION_TIMEOUT_MS = 30_000;

export function OptimisticTradeUI() {
  const [balance, setBalance] = useState(1000);
  const [pendingTxHash, setPendingTxHash] = useState(null);
  const [state, send] = useMachine(tradeMachine);
  const timeoutRef = useRef(null);
  
  // Use TransactionContext for real-time transaction status updates
  const { addPending, getPendingByHash } = useTransactions();

  // Subscribe only to events relevant to trade confirmation / failure.
  const { events } = usePifpEvents({
    event_types: ['project_funded', 'funds_released', 'project_expired', 'project_cancelled'],
  });

  // Watch incoming events for confirmation or rollback of the pending trade.
  useEffect(() => {
    if (!state.matches('optimistic') || !pendingTxHash) return;

    const latest = events[0];
    if (!latest) return;

    const isMatch =
      latest.tx_hash === pendingTxHash ||
      // Fallback: accept any confirmation event while we have a pending trade.
      ['project_funded', 'funds_released'].includes(latest.event_type);

    const isFailure = ['project_expired', 'project_cancelled'].includes(
      latest.event_type,
    );

    if (isMatch && !isFailure) {
      clearTimeout(timeoutRef.current);
      send('CONFIRM');
    } else if (isFailure) {
      clearTimeout(timeoutRef.current);
      send('REJECT');
      setBalance((b) => b + 100);
    }
  }, [events, pendingTxHash, state, send]);

  // Also check TransactionContext for real-time tx status updates
  useEffect(() => {
    if (!pendingTxHash) return;
    
    const checkTxStatus = () => {
      const pendingTx = getPendingByHash(pendingTxHash);
      if (!pendingTx) return;
      
      if (pendingTx.status === 'confirmed') {
        clearTimeout(timeoutRef.current);
        send('CONFIRM');
      } else if (pendingTx.status === 'failed') {
        clearTimeout(timeoutRef.current);
        send('REJECT');
        setBalance((b) => b + 100);
      }
    };
    
    // Poll for status changes (WebSocket will update the context)
    const interval = setInterval(checkTxStatus, 1000);
    return () => clearInterval(interval);
  }, [pendingTxHash, getPendingByHash, send]);

  const handleSubmit = () => {
    // Optimistically deduct balance before on-chain confirmation.
    setBalance((b) => b - 100);
    // Generate a placeholder tx hash; in production this comes from the wallet.
    const mockTxHash = `0x${Math.random().toString(16).slice(2, 18)}`;
    setPendingTxHash(mockTxHash);
    send('SUBMIT');

    // Register with TransactionContext for real-time updates
    addPending({
      id: '', // Will be generated
      txHash: mockTxHash,
      amount: 100,
      type: 'trade',
    });

    // Safety timeout: roll back if no confirmation arrives in time.
    timeoutRef.current = setTimeout(() => {
      if (!state.matches('optimistic')) return;
      send('REJECT');
      setBalance((b) => b + 100);
    }, CONFIRMATION_TIMEOUT_MS);
  };

  // Clean up timeout on unmount.
  useEffect(() => () => clearTimeout(timeoutRef.current), []);

  const isOptimistic = state.matches('optimistic');
  const isConfirmed = state.matches('confirmed');
  const isRolledBack = state.matches('rolledBack');

  return (
    <div className="optimistic-trade">
      <h2>Optimistic P2P Trade</h2>
      <p>
        Balance:{' '}
        <motion.span
          animate={{
            scale: isOptimistic ? 1.1 : 1,
            color: isRolledBack ? '#ff4444' : isConfirmed ? '#4ade80' : '#f1f5f9',
          }}
          transition={{ duration: 0.3 }}
        >
          {balance}
        </motion.span>
      </p>

      {isOptimistic && (
        <p className="pending">
          ⟳ Awaiting on-chain confirmation…
          {pendingTxHash && (
            <span className="tx-hash"> ({pendingTxHash.slice(0, 10)}…)</span>
          )}
        </p>
      )}

      <button onClick={handleSubmit} disabled={!state.matches('idle')}>
        Submit Trade
      </button>

      <p>
        State:{' '}
        <motion.span
          key={String(state.value)}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.2 }}
        >
          {String(state.value)}
        </motion.span>
      </p>

      {isRolledBack && (
        <p className="error">Transaction failed — balance restored.</p>
      )}
      {isConfirmed && (
        <p className="success">Transaction confirmed on-chain.</p>
      )}

      <style>{`
        .optimistic-trade { color: #f1f5f9; }
        .pending { color: #60a5fa; font-size: 0.9rem; }
        .tx-hash { font-family: monospace; font-size: 0.8rem; color: #94a3b8; }
        .error { color: #f87171; }
        .success { color: #4ade80; }
      `}</style>
    </div>
  );
}
