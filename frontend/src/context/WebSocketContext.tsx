import React, { createContext, useContext, useEffect, useRef } from 'react';
import { RingBuffer, EventType } from '../lib/ringBuffer.js';

interface WebSocketContextValue {
  log: (type: number, payload: object) => void;
}

export const WebSocketContext = createContext<WebSocketContextValue>({
  log: () => {},
});

export const useLogger = () => useContext(WebSocketContext);

export const WebSocketProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const ringRef   = useRef<RingBuffer | null>(null);
  const workerRef = useRef<Worker | null>(null);

  useEffect(() => {
    // Boot the ring buffer and hand the SAB to the worker
    const ring   = new RingBuffer();
    ringRef.current = ring;

    const worker = new Worker(
      new URL('../workers/logger.worker.js', import.meta.url),
      { type: 'module' }
    );
    workerRef.current = worker;

    worker.postMessage({ type: 'init', sab: ring.buffer });

    worker.onmessage = (e) => {
      if (e.data.type === 'flush') {
        // Events flushed from worker — hook into your server send here
        console.debug('[Logger] flushed', e.data.events.length, 'events', e.data.events);
      }
    };

    return () => worker.terminate();
  }, []);

  const log = (type: number, payload: object) => {
    ringRef.current?.write(type, payload);
  };

  return (
    <WebSocketContext.Provider value={{ log }}>
      {children}
    </WebSocketContext.Provider>
  );
};

export { EventType };
