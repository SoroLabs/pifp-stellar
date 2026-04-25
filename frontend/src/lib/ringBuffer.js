// ─── Layout of the SharedArrayBuffer ───────────────────────────────────────
// [0]  writeHead  (Atomics int32) — next slot to write into
// [1]  readHead   (Atomics int32) — next slot to flush from
// [2+] data slots — each slot is SLOT_SIZE bytes
// ───────────────────────────────────────────────────────────────────────────

const SLOT_COUNT = 4096;
const SLOT_SIZE  = 256;
const META_INTS  = 2;
const META_BYTES = META_INTS * 4;
const BUFFER_SIZE = META_BYTES + SLOT_COUNT * SLOT_SIZE;

// ─── Event type codes ───────────────────────────────────────────────────────
export const EventType = {
  TRANSACTION: 0x01,
  NETWORK:     0x02,
  CLICK:       0x03,
  ERROR:       0x04,
};

// ─── Binary Serializer ──────────────────────────────────────────────────────
// Slot format:
//  [0]      type       uint8
//  [1..8]   timestamp  float64 (little-endian)
//  [9..12]  payloadLen uint32
//  [13..]   payload    UTF-8 JSON (truncated to fit)

function serialize(type, payload) {
  const bytes = new Uint8Array(SLOT_SIZE);
  const view  = new DataView(bytes.buffer);
  bytes[0] = type & 0xff;
  view.setFloat64(1, Date.now(), true);
  const encoded = new TextEncoder().encode(JSON.stringify(payload));
  const chunk   = encoded.slice(0, SLOT_SIZE - 13);
  view.setUint32(9, chunk.byteLength, true);
  bytes.set(chunk, 13);
  return bytes;
}

export function deserialize(bytes) {
  const view       = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const type       = bytes[0];
  const timestamp  = view.getFloat64(1, true);
  const payloadLen = view.getUint32(9, true);
  const json       = new TextDecoder().decode(bytes.slice(13, 13 + payloadLen));
  let payload;
  try { payload = JSON.parse(json); } catch { payload = {}; }
  return { type, timestamp, payload };
}

// ─── RingBuffer ─────────────────────────────────────────────────────────────
export class RingBuffer {
  constructor(sab) {
    this.sab  = sab ?? new SharedArrayBuffer(BUFFER_SIZE);
    this.meta = new Int32Array(this.sab, 0, META_INTS);
    this.data = new Uint8Array(this.sab, META_BYTES);
  }

  // Main thread — lock-free write
  write(type, payload) {
    const slot   = Atomics.load(this.meta, 0) % SLOT_COUNT;
    const offset = slot * SLOT_SIZE;
    this.data.set(serialize(type, payload), offset);
    Atomics.add(this.meta, 0, 1);
  }

  // Worker — drain all unread slots
  drain() {
    const writeHead = Atomics.load(this.meta, 0);
    const readHead  = Atomics.load(this.meta, 1);
    if (writeHead === readHead) return [];

    const events = [];
    for (let i = readHead; i < writeHead; i++) {
      const offset = (i % SLOT_COUNT) * SLOT_SIZE;
      events.push(deserialize(this.data.slice(offset, offset + SLOT_SIZE)));
    }
    Atomics.store(this.meta, 1, writeHead);
    return events;
  }

  get buffer() { return this.sab; }
}
