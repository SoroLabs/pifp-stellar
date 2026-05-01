/**
 * usePifpEvents — subscribe to PIFP contract events from the WebSocket stream.
 *
 * @param filter  Optional subscription filter. Changing this re-sends the
 *                filter to the server so only matching events are delivered.
 * @param maxItems  Maximum number of events to keep in the local buffer (default 50).
 *
 * @example
 * const { events, status } = usePifpEvents({ event_types: ['project_funded'] });
 */

import { useEffect, useRef, useState } from 'react';
import { useWebSocket } from '../context/WebSocketContext';
import type { ConnectionStatus, PifpEvent, SubscriptionFilter } from '../types/events';

interface UsePifpEventsResult {
  events: PifpEvent[];
  status: ConnectionStatus;
  clearEvents: () => void;
}

export function usePifpEvents(
  filter: SubscriptionFilter = {},
  maxItems = 50,
): UsePifpEventsResult {
  const { status, setFilter, addListener } = useWebSocket();
  const [events, setEvents] = useState<PifpEvent[]>([]);
  // Stable ref so the listener closure doesn't capture a stale maxItems.
  const maxRef = useRef(maxItems);
  maxRef.current = maxItems;

  // Apply filter whenever it changes (deep-compare via JSON).
  const filterKey = JSON.stringify(filter);
  useEffect(() => {
    setFilter(filter);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filterKey, setFilter]);

  // Register event listener.
  useEffect(() => {
    const remove = addListener((event: PifpEvent) => {
      setEvents((prev) => [event, ...prev].slice(0, maxRef.current));
    });
    return remove;
  }, [addListener]);

  const clearEvents = () => setEvents([]);

  return { events, status, clearEvents };
}
