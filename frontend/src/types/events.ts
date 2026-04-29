/**
 * TypeScript types mirroring the backend `PifpEvent` and WebSocket wire protocol.
 */

// ─── Event kinds ──────────────────────────────────────────────────────────────

export type EventKind =
  | 'project_created'
  | 'project_funded'
  | 'project_active'
  | 'project_verified'
  | 'project_expired'
  | 'project_cancelled'
  | 'funds_released'
  | 'donator_refunded'
  | 'role_set'
  | 'role_del'
  | 'protocol_paused'
  | 'protocol_unpaused'  | 'tx_pending'
  | 'tx_confirmed'
  | 'tx_failed'  | 'unknown';

// ─── Core event payload ───────────────────────────────────────────────────────

export interface PifpEvent {
  event_type: EventKind;
  project_id: string | null;
  actor: string | null;
  amount: string | null;
  ledger: number;
  timestamp: number;
  contract_id: string;
  tx_hash: string | null;
  extra_data: string | null;
}

// ─── WebSocket wire protocol ──────────────────────────────────────────────────

/** Messages the server sends to the client. */
export type ServerMessage =
  | { type: 'connected'; message: string }
  | { type: 'event'; payload: PifpEvent }
  | { type: 'transaction_update'; payload: TransactionUpdate }
  | { type: 'pong' };

/** Transaction status update payload. */
export interface TransactionUpdate {
  /** The transaction hash being updated. */
  tx_hash: string;
  /** Current status of the transaction. */
  status: TransactionStatus;
  /** Optional project ID if this tx is related to a project. */
  project_id?: string;
  /** Ledger number when status changed. */
  ledger: number;
  /** Error message if status is failed. */
  error_message?: string;
}

/** Transaction status enum. */
export type TransactionStatus = 'pending' | 'confirmed' | 'failed';

/** Messages the client sends to the server. */
export type ClientMessage =
  | {
      type: 'subscribe';
      /** Empty array = accept all event types. */
      event_types?: EventKind[];
      /** Empty array = accept all project IDs. */
      project_ids?: string[];
    }
  | { type: 'ping' };

// ─── Subscription filter (client-side mirror) ─────────────────────────────────

export interface SubscriptionFilter {
  event_types?: EventKind[];
  project_ids?: string[];
}

// ─── Connection state ─────────────────────────────────────────────────────────

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';
