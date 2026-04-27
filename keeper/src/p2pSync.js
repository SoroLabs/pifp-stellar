import { loadTaskRegistry, saveTaskRegistry } from "./taskRegistry.js";

const GOSSIP_INTERVAL_MS = parseInt(process.env.P2P_GOSSIP_INTERVAL_MS || "15000", 10);
const GOSSIP_TIMEOUT_MS = parseInt(process.env.P2P_GOSSIP_TIMEOUT_MS || "5000", 10);

let nodeId = "";
let peers = new Map();
let gossipTimer = null;

export function initP2P(options = {}) {
  nodeId = options.nodeId || `keeper-${Math.random().toString(36).slice(2, 10)}`;
  peers = new Map();

  const bootstrapEnv = process.env.P2P_BOOTSTRAP_PEERS || "";
  const bootstrapUrls = bootstrapEnv.split(",").map((u) => u.trim()).filter(Boolean);
  for (const url of bootstrapUrls) {
    registerPeer(url);
  }
  if (options.bootstrapPeers) {
    for (const url of options.bootstrapPeers) {
      registerPeer(url);
    }
  }
}

export function getNodeId() {
  return nodeId;
}

export function getPeers() {
  return Array.from(peers.values());
}

export function registerPeer(url) {
  if (!url || typeof url !== "string") return null;
  const trimmed = url.trim();
  if (!trimmed) return null;
  const id = trimmed.replace(/[^a-zA-Z0-9]/g, "_").slice(-24);
  if (!peers.has(id)) {
    peers.set(id, { id, url: trimmed, lastSeen: null, latencyMs: null, status: "unknown" });
  }
  return peers.get(id);
}

export async function gossipToPeer(peerUrl) {
  const id = peerUrl.replace(/[^a-zA-Z0-9]/g, "_").slice(-24);
  const peer = peers.get(id) || registerPeer(peerUrl);
  const tasks = loadTaskRegistry();
  const payload = { nodeId, tasks, timestamp: Date.now() };
  const start = Date.now();
  try {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), GOSSIP_TIMEOUT_MS);
    const resp = await fetch(`${peerUrl}/peers/gossip`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      signal: ctrl.signal,
    });
    clearTimeout(timer);
    const latencyMs = Date.now() - start;
    peer.latencyMs = latencyMs;
    peer.lastSeen = new Date().toISOString();
    peer.status = resp.ok ? "connected" : "degraded";
    if (resp.ok) {
      const remote = await resp.json();
      mergeRemoteState(remote);
    }
  } catch {
    const latencyMs = Date.now() - start;
    peer.latencyMs = latencyMs;
    peer.status = latencyMs >= GOSSIP_TIMEOUT_MS ? "degraded" : "unreachable";
  }
  return peer;
}

export async function gossipAll() {
  const results = [];
  for (const peer of peers.values()) {
    results.push(await gossipToPeer(peer.url));
  }
  return results;
}

export function mergeRemoteState(remote) {
  if (!remote || !Array.isArray(remote.tasks)) return;
  const tasks = loadTaskRegistry();
  const byId = new Map(tasks.map((t) => [t.id, t]));
  let changed = false;
  for (const rt of remote.tasks) {
    if (!rt.id) continue;
    const local = byId.get(rt.id);
    if (!local || (rt.updatedAt && (!local.updatedAt || rt.updatedAt > local.updatedAt))) {
      byId.set(rt.id, { ...rt });
      changed = true;
    }
  }
  if (changed) {
    saveTaskRegistry(Array.from(byId.values()));
  }
}

export function handleIncomingGossip(payload) {
  if (payload && payload.nodeId && payload.nodeId !== nodeId) {
    mergeRemoteState(payload);
    if (payload.nodeId) {
      registerPeer(payload.originUrl || `peer-${payload.nodeId}`);
    }
  }
  const tasks = loadTaskRegistry();
  return { nodeId, tasks, timestamp: Date.now() };
}

export function startGossipLoop() {
  if (gossipTimer) clearInterval(gossipTimer);
  gossipTimer = setInterval(() => {
    gossipAll().catch(() => {});
  }, GOSSIP_INTERVAL_MS);
}

export function stopGossipLoop() {
  if (gossipTimer) {
    clearInterval(gossipTimer);
    gossipTimer = null;
  }
}
