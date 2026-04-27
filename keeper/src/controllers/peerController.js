import { loadTaskRegistry } from "../taskRegistry.js";
import { getPeers, getNodeId, registerPeer } from "../p2pSync.js";

export function listPeers(req, res) {
  const peers = getPeers();
  res.json({ nodeId: getNodeId(), peerCount: peers.length, peers });
}

export function getPeerStatus(req, res) {
  const peers = getPeers();
  const tasks = loadTaskRegistry();
  const connected = peers.filter((p) => p.status === "connected").length;
  const degraded = peers.filter((p) => p.status === "degraded").length;
  const unreachable = peers.filter((p) => p.status === "unreachable").length;
  const latencies = peers.map((p) => p.latencyMs).filter((l) => l != null);
  const avgLatencyMs = latencies.length
    ? Math.round(latencies.reduce((a, b) => a + b, 0) / latencies.length)
    : null;
  res.json({
    nodeId: getNodeId(),
    uptime: process.uptime(),
    timestamp: new Date().toISOString(),
    network: {
      totalPeers: peers.length,
      connected,
      degraded,
      unreachable,
      avgLatencyMs,
    },
    tasks: {
      total: tasks.length,
      active: tasks.filter((t) => t.status === "active").length,
      completed: tasks.filter((t) => t.status === "completed").length,
    },
    peers,
  });
}

export function addPeer(req, res) {
  const { url } = req.body || {};
  if (!url || typeof url !== "string") {
    return res.status(400).json({ error: "url is required" });
  }
  let parsed;
  try {
    parsed = new URL(url);
  } catch {
    return res.status(400).json({ error: "Invalid URL" });
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return res.status(400).json({ error: "URL must use http or https" });
  }
  const peer = registerPeer(url);
  if (!peer) {
    return res.status(400).json({ error: "Failed to register peer" });
  }
  res.status(201).json({ peer });
}
