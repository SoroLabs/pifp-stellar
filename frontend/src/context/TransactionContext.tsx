/**
 * TransactionContext — manages pending transactions with real-time WebSocket updates.
 *
 * Features:
 * - Tracks pending transactions with optimistic state
 * - Updates transaction status based on WebSocket transaction_update messages
 * - Provides automatic rollback on timeout or failure
 * - Integrates with WebSocketContext for real-time updates
 */

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useReducer,
  useRef,
} from "react";
import { useWebSocket } from "./WebSocketContext";
import type { TransactionStatus, TransactionUpdate } from "../types/events";

// ─── Transaction state ───────────────────────────────────────────────────────

export interface PendingTransaction {
  /** Unique identifier for the transaction. */
  id: string;
  /** The transaction hash from the blockchain. */
  txHash: string;
  /** Current status of the transaction. */
  status: TransactionStatus;
  /** Amount involved in the transaction (for rollback). */
  amount: number;
  /** Type of transaction for UI display. */
  type: "deposit" | "withdraw" | "trade" | "fund" | "release";
  /** Related project ID if applicable. */
  projectId?: string;
  /** Timestamp when the transaction was submitted. */
  submittedAt: number;
  /** Error message if status is 'failed'. */
  errorMessage?: string;
}

interface TransactionState {
  /** All pending transactions. */
  pending: Map<string, PendingTransaction>;
  /** Transaction history (confirmed/failed). */
  history: PendingTransaction[];
}

type TransactionAction =
  | { type: "ADD_PENDING"; payload: PendingTransaction }
  | {
      type: "UPDATE_STATUS";
      payload: { id: string; status: TransactionStatus; errorMessage?: string };
    }
  | { type: "REMOVE_PENDING"; payload: string }
  | { type: "MOVE_TO_HISTORY"; payload: PendingTransaction }
  | { type: "CLEAR_HISTORY" };

function transactionReducer(
  state: TransactionState,
  action: TransactionAction,
): TransactionState {
  switch (action.type) {
    case "ADD_PENDING": {
      const newPending = new Map(state.pending);
      newPending.set(action.payload.id, action.payload);
      return { ...state, pending: newPending };
    }
    case "UPDATE_STATUS": {
      const tx = state.pending.get(action.payload.id);
      if (!tx) return state;
      const updatedTx = {
        ...tx,
        status: action.payload.status,
        errorMessage: action.payload.errorMessage,
      };
      const newPending = new Map(state.pending);
      newPending.set(action.payload.id, updatedTx);
      return { ...state, pending: newPending };
    }
    case "REMOVE_PENDING": {
      const newPending = new Map(state.pending);
      newPending.delete(action.payload);
      return { ...state, pending: newPending };
    }
    case "MOVE_TO_HISTORY": {
      const newPending = new Map(state.pending);
      newPending.delete(action.payload.id);
      return {
        ...state,
        pending: newPending,
        history: [action.payload, ...state.history].slice(0, 100),
      };
    }
    case "CLEAR_HISTORY":
      return { ...state, history: [] };
    default:
      return state;
  }
}

// ─── Config ───────────────────────────────────────────────────────────────────

const DEFAULT_TIMEOUT_MS = 60_000; // 60 seconds for transaction confirmation

// ─── Context value ────────────────────────────────────────────────────────────

export interface TransactionContextValue {
  /** Add a new pending transaction with optimistic state. */
  addPending: (
    tx: Omit<PendingTransaction, "status" | "submittedAt">,
  ) => string;
  /** Get a pending transaction by ID. */
  getPending: (id: string) => PendingTransaction | undefined;
  /** Get a pending transaction by tx hash. */
  getPendingByHash: (hash: string) => PendingTransaction | undefined;
  /** Get all pending transactions. */
  getAllPending: () => PendingTransaction[];
  /** Get transaction history. */
  getHistory: () => PendingTransaction[];
  /** Remove a pending transaction (e.g., after manual resolution). */
  removePending: (id: string) => void;
  /** Clear transaction history. */
  clearHistory: () => void;
}

export const TransactionContext = createContext<TransactionContextValue>({
  addPending: () => "",
  getPending: () => undefined,
  getPendingByHash: () => undefined,
  getAllPending: () => [],
  getHistory: () => [],
  removePending: () => {},
  clearHistory: () => {},
});

export const useTransactions = () => useContext(TransactionContext);

// ─── Provider ─────────────────────────────────────────────────────────────────

interface TransactionProviderProps {
  children: React.ReactNode;
  /** Timeout in ms before a pending transaction is considered failed. Default: 60000 */
  timeoutMs?: number;
}

export const TransactionProvider: React.FC<TransactionProviderProps> = ({
  children,
  timeoutMs = DEFAULT_TIMEOUT_MS,
}) => {
  const [state, dispatch] = useReducer(transactionReducer, {
    pending: new Map(),
    history: [],
  });

  const { addTransactionListener } = useWebSocket();
  const timeoutRefsRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(
    new Map(),
  );

  // ── Handle incoming transaction updates from WebSocket ─────────────────────
  useEffect(() => {
    const cleanup = addTransactionListener((update: TransactionUpdate) => {
      // Find pending transaction by tx hash
      const pending = Array.from(state.pending.values()).find(
        (tx) => tx.txHash === update.tx_hash,
      );

      if (pending) {
        // Clear timeout since we got an update
        const timeout = timeoutRefsRef.current.get(pending.id);
        if (timeout) {
          clearTimeout(timeout);
          timeoutRefsRef.current.delete(pending.id);
        }

        // Update status
        dispatch({
          type: "UPDATE_STATUS",
          payload: {
            id: pending.id,
            status: update.status,
            errorMessage: update.error_message,
          },
        });

        // Move to history if confirmed or failed
        if (update.status === "confirmed" || update.status === "failed") {
          const updatedTx: PendingTransaction = {
            ...pending,
            status: update.status,
            errorMessage: update.error_message,
          };
          dispatch({ type: "MOVE_TO_HISTORY", payload: updatedTx });
        }
      }
    });

    return cleanup;
  }, [addTransactionListener, state.pending]);

  // ── Add a new pending transaction ─────────────────────────────────────────
  const addPending = useCallback(
    (tx: Omit<PendingTransaction, "status" | "submittedAt">): string => {
      const id = `tx_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
      const pendingTx: PendingTransaction = {
        ...tx,
        id,
        status: "pending",
        submittedAt: Date.now(),
      };

      dispatch({ type: "ADD_PENDING", payload: pendingTx });

      // Set up timeout for automatic rollback
      const timeout = setTimeout(() => {
        dispatch({
          type: "UPDATE_STATUS",
          payload: {
            id,
            status: "failed",
            errorMessage: "Transaction timed out",
          },
        });
        const failedTx: PendingTransaction = {
          ...pendingTx,
          status: "failed",
          errorMessage: "Transaction timed out",
        };
        dispatch({ type: "MOVE_TO_HISTORY", payload: failedTx });
      }, timeoutMs);

      timeoutRefsRef.current.set(id, timeout);
      return id;
    },
    [timeoutMs],
  );

  // ── Get pending transaction by ID ─────────────────────────────────────────
  const getPending = useCallback(
    (id: string): PendingTransaction | undefined => {
      return state.pending.get(id);
    },
    [state.pending],
  );

  // ── Get pending transaction by tx hash ────────────────────────────────────
  const getPendingByHash = useCallback(
    (hash: string): PendingTransaction | undefined => {
      return Array.from(state.pending.values()).find(
        (tx) => tx.txHash === hash,
      );
    },
    [state.pending],
  );

  // ── Get all pending transactions ──────────────────────────────────────────
  const getAllPending = useCallback((): PendingTransaction[] => {
    return Array.from(state.pending.values());
  }, [state.pending]);

  // ── Get transaction history ───────────────────────────────────────────────
  const getHistory = useCallback((): PendingTransaction[] => {
    return state.history;
  }, [state.history]);

  // ── Remove a pending transaction ─────────────────────────────────────────
  const removePending = useCallback((id: string) => {
    const timeout = timeoutRefsRef.current.get(id);
    if (timeout) {
      clearTimeout(timeout);
      timeoutRefsRef.current.delete(id);
    }
    dispatch({ type: "REMOVE_PENDING", payload: id });
  }, []);

  // ── Clear history ─────────────────────────────────────────────────────────
  const clearHistory = useCallback(() => {
    dispatch({ type: "CLEAR_HISTORY" });
  }, []);

  // ── Cleanup timeouts on unmount ───────────────────────────────────────────
  useEffect(() => {
    return () => {
      timeoutRefsRef.current.forEach((timeout) => clearTimeout(timeout));
      timeoutRefsRef.current.clear();
    };
  }, []);

  return (
    <TransactionContext.Provider
      value={{
        addPending,
        getPending,
        getPendingByHash,
        getAllPending,
        getHistory,
        removePending,
        clearHistory,
      }}
    >
      {children}
    </TransactionContext.Provider>
  );
};
