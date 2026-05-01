/**
 * WebSocketContext — persistent connection to the PIFP event stream.
 *
 * Features:
 * - Connects to the backend WebSocket server (VITE_WS_URL or ws://localhost:9001)
 * - Exponential backoff reconnection (1 s → 2 s → 4 s … capped at 30 s)
 * - Per-client subscription filtering (event_types, project_ids)
 * - Cross-tab leader election: only the leader tab holds the WS connection;
 *   follower tabs receive events via BroadcastChannel
 * - Heartbeat ping every 25 s to keep the connection alive through proxies
 */

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import type {
  ClientMessage,
  ConnectionStatus,
  PifpEvent,
  ServerMessage,
  SubscriptionFilter,
  TransactionUpdate,
} from "../types/events";

// ─── Config ───────────────────────────────────────────────────────────────────

const WS_URL =
  (import.meta.env.VITE_WS_URL as string | undefined) ?? "ws://localhost:9001";

const HEARTBEAT_MS = 25_000;
const INITIAL_BACKOFF_MS = 1_000;
const MAX_BACKOFF_MS = 30_000;
const LEADER_CHANNEL = "pifp-ws-leader";
const EVENT_CHANNEL = "pifp-ws-events";

// ─── Context value ────────────────────────────────────────────────────────────

export interface WebSocketContextValue {
  status: ConnectionStatus;
  isLeader: boolean;
  /** Subscribe to a subset of events. Pass `{}` to receive everything. */
  setFilter: (filter: SubscriptionFilter) => void;
  /** Latest event received (useful for simple consumers). */
  lastEvent: PifpEvent | null;
  /** Register a listener that fires for every incoming event. Returns a cleanup fn. */
  addListener: (fn: (event: PifpEvent) => void) => () => void;
  /** Register a listener for transaction status updates. Returns a cleanup fn. */
  addTransactionListener: (
    fn: (update: TransactionUpdate) => void,
  ) => () => void;
}

export const WebSocketContext = createContext<WebSocketContextValue>({
  status: "disconnected",
  isLeader: false,
  setFilter: () => {},
  lastEvent: null,
  addListener: () => () => {},
  addTransactionListener: () => () => {},
});

export const useWebSocket = () => useContext(WebSocketContext);

// ─── Provider ─────────────────────────────────────────────────────────────────

export const WebSocketProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [isLeader, setIsLeader] = useState(false);
  const [lastEvent, setLastEvent] = useState<PifpEvent | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const filterRef = useRef<SubscriptionFilter>({});
  const backoffRef = useRef(INITIAL_BACKOFF_MS);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heartbeatTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const listenersRef = useRef<Set<(e: PifpEvent) => void>>(new Set());
  const txListenersRef = useRef<Set<(e: TransactionUpdate) => void>>(new Set());
  const leaderChannelRef = useRef<BroadcastChannel | null>(null);
  const eventChannelRef = useRef<BroadcastChannel | null>(null);
  const isLeaderRef = useRef(false);

  // ── Emit to all registered listeners ──────────────────────────────────────
  const emit = useCallback((event: PifpEvent) => {
    setLastEvent(event);
    listenersRef.current.forEach((fn) => fn(event));
  }, []);

  // ── Emit transaction update to all registered listeners ───────────────────
  const emitTransactionUpdate = useCallback((update: TransactionUpdate) => {
    txListenersRef.current.forEach((fn) => fn(update));
  }, []);

  // ── Send a message to the server (no-op if not connected) ─────────────────
  const send = useCallback((msg: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  }, []);

  // ── Apply the current filter to the server ─────────────────────────────────
  const applyFilter = useCallback(() => {
    send({ type: "subscribe", ...filterRef.current });
  }, [send]);

  // ── Stop heartbeat ─────────────────────────────────────────────────────────
  const stopHeartbeat = () => {
    if (heartbeatTimerRef.current !== null) {
      clearInterval(heartbeatTimerRef.current);
      heartbeatTimerRef.current = null;
    }
  };

  // ── Start heartbeat ────────────────────────────────────────────────────────
  const startHeartbeat = useCallback(() => {
    stopHeartbeat();
    heartbeatTimerRef.current = setInterval(() => {
      send({ type: "ping" });
    }, HEARTBEAT_MS);
  }, [send]);

  // ── Connect (leader only) ──────────────────────────────────────────────────
  const connect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    setStatus("connecting");
    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      setStatus("connected");
      backoffRef.current = INITIAL_BACKOFF_MS;
      applyFilter();
      startHeartbeat();
    };

    ws.onmessage = (e: MessageEvent<string>) => {
      let msg: ServerMessage;
      try {
        msg = JSON.parse(e.data) as ServerMessage;
      } catch {
        return;
      }

      if (msg.type === "event") {
        emit(msg.payload);
        // Relay to follower tabs.
        eventChannelRef.current?.postMessage(msg.payload);
      } else if (msg.type === "transaction_update") {
        // Emit transaction updates to listeners.
        emitTransactionUpdate(msg.payload);
      }
    };

    ws.onerror = () => {
      setStatus("error");
    };

    ws.onclose = () => {
      stopHeartbeat();
      setStatus("disconnected");
      wsRef.current = null;

      // Exponential backoff reconnect.
      const delay = backoffRef.current;
      backoffRef.current = Math.min(delay * 2, MAX_BACKOFF_MS);
      reconnectTimerRef.current = setTimeout(connect, delay);
    };
  }, [applyFilter, emit, startHeartbeat]);

  // ── Disconnect ─────────────────────────────────────────────────────────────
  const disconnect = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    stopHeartbeat();
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    setStatus("disconnected");
  }, []);

  // ── Leader election via BroadcastChannel ──────────────────────────────────
  useEffect(() => {
    const leaderCh = new BroadcastChannel(LEADER_CHANNEL);
    const eventCh = new BroadcastChannel(EVENT_CHANNEL);
    leaderChannelRef.current = leaderCh;
    eventChannelRef.current = eventCh;

    // Follower tabs receive relayed events from the leader.
    eventCh.onmessage = (e: MessageEvent<PifpEvent>) => {
      if (!isLeaderRef.current) {
        emit(e.data);
      }
    };

    // Simple leader election: announce candidacy; if no existing leader
    // responds within 100 ms, claim leadership.
    leaderCh.postMessage({ type: "candidate" });

    const claimTimer = setTimeout(() => {
      isLeaderRef.current = true;
      setIsLeader(true);
      leaderCh.postMessage({ type: "leader" });
      connect();
    }, 100);

    leaderCh.onmessage = (e: MessageEvent<{ type: string }>) => {
      if (e.data.type === "leader" && !isLeaderRef.current) {
        // Another tab is already the leader — stay as follower.
        clearTimeout(claimTimer);
      }
      if (e.data.type === "candidate" && isLeaderRef.current) {
        // Respond to new tabs so they know a leader exists.
        leaderCh.postMessage({ type: "leader" });
      }
    };

    return () => {
      clearTimeout(claimTimer);
      if (isLeaderRef.current) {
        disconnect();
      }
      leaderCh.close();
      eventCh.close();
    };
  }, [connect, disconnect, emit]);

  // ── Public API ─────────────────────────────────────────────────────────────

  const setFilter = useCallback(
    (filter: SubscriptionFilter) => {
      filterRef.current = filter;
      if (isLeaderRef.current) {
        applyFilter();
      }
    },
    [applyFilter],
  );

  const addListener = useCallback((fn: (event: PifpEvent) => void) => {
    listenersRef.current.add(fn);
    return () => {
      listenersRef.current.delete(fn);
    };
  }, []);

  const addTransactionListener = useCallback(
    (fn: (update: TransactionUpdate) => void) => {
      txListenersRef.current.add(fn);
      return () => {
        txListenersRef.current.delete(fn);
      };
    },
    [],
  );

  return (
    <WebSocketContext.Provider
      value={{
        status,
        isLeader,
        setFilter,
        lastEvent,
        addListener,
        addTransactionListener,
      }}
    >
      {children}
    </WebSocketContext.Provider>
  );
};
