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
  | 'protocol_unpaused'
  | 'unknown';

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
  | { type: 'pong' };

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
