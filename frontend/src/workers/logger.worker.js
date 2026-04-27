import { RingBuffer } from '../lib/ringBuffer.js';

let ring = null;

// Receive the SharedArrayBuffer from the main thread
self.onmessage = (e) => {
  if (e.data.type === 'init') {
    ring = new RingBuffer(e.data.sab);
    startFlushing();
  }
};

function startFlushing() {
  setInterval(() => {
    if (!ring) return;

    const events = ring.drain();
    if (events.length === 0) return;

    // Post drained events back to main thread (or send to server here)
    self.postMessage({ type: 'flush', events });

    // TODO: replace postMessage with a direct fetch/WebSocket send to server
    // fetch('/api/logs', { method: 'POST', body: JSON.stringify(events) })
  }, 500);
}
