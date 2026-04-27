import { useEffect, useRef, useState } from "react";

const KEEPER_URL = (import.meta.env.VITE_KEEPER_URL || "http://localhost:3000").replace(/\/$/, "");
const POLL_INTERVAL_MS = 10000;

const STATUS_COLOR = { connected: "#22c55e", degraded: "#f59e0b", unreachable: "#ef4444", unknown: "#94a3b8" };

function MetricCard({ label, value, unit }) {
  return (
    <div className="metric-card">
      <span className="metric-label">{label}</span>
      <span className="metric-value">{value !== null && value !== undefined ? value : "-"}</span>
      {unit && <span className="metric-unit">{unit}</span>}
    </div>
  );
}

function NetworkGraph({ selfId, peers }) {
  const SIZE = 320;
  const CX = SIZE / 2;
  const CY = SIZE / 2;
  const R = 110;
  const SELF_R = 18;
  const PEER_R = 12;
  const angleStep = peers.length > 0 ? (2 * Math.PI) / peers.length : 0;

  return (
    <div className="network-graph-wrap">
      <svg className="network-svg" viewBox={"0 0 " + SIZE + " " + SIZE} aria-label="Network mesh graph">
        {peers.map((peer, i) => {
          const angle = i * angleStep - Math.PI / 2;
          const px = CX + R * Math.cos(angle);
          const py = CY + R * Math.sin(angle);
          const color = STATUS_COLOR[peer.status] || STATUS_COLOR.unknown;
          const isDashed = peer.status === "degraded" || peer.status === "unreachable";
          return (
            <g key={peer.id || peer.url}>
              <line x1={CX} y1={CY} x2={px} y2={py} stroke={color} strokeWidth={1.5} strokeDasharray={isDashed ? "6 4" : undefined} opacity={0.7} />
              <circle cx={px} cy={py} r={PEER_R} fill={color} opacity={0.9} />
              <title>{peer.url + " -- " + peer.status + (peer.latencyMs != null ? " (" + peer.latencyMs + "ms)" : "")}</title>
            </g>
          );
        })}
        <circle cx={CX} cy={CY} r={SELF_R} fill="#6366f1" />
        <text x={CX} y={CY + 4} textAnchor="middle" fontSize="9" fill="white" fontFamily="monospace">
          {selfId ? selfId.slice(0, 10) : "self"}
        </text>
      </svg>
    </div>
  );
}

export function NetworkDashboard() {
  const [data, setData] = useState(null);
  const [error, setError] = useState("");
  const timerRef = useRef(null);

  async function fetchStatus() {
    try {
      const resp = await fetch(KEEPER_URL + "/peers/status");
      if (!resp.ok) throw new Error("HTTP " + resp.status);
      const json = await resp.json();
      setData(json);
      setError("");
    } catch (err) {
      setError(err.message || "Failed to reach keeper");
    }
  }

  useEffect(() => {
    fetchStatus();
    timerRef.current = setInterval(fetchStatus, POLL_INTERVAL_MS);
    return () => clearInterval(timerRef.current);
  }, []);

  const net = data ? data.network : {};
  const tasks = data ? data.tasks : {};
  const peers = data ? data.peers : [];

  return (
    <section className="network-dashboard" aria-label="P2P Network Dashboard">
      <header>
        <h2>P2P Network Dashboard</h2>
        <p className="subhead">Live view of the keeper gossip mesh. Refreshes every 10 s.</p>
      </header>

      {error && <p className="state error" role="alert">{error}</p>}

      <div className="metric-grid">
        <MetricCard label="Node ID" value={data && data.nodeId ? data.nodeId.slice(0, 16) : "-"} />
        <MetricCard label="Total Peers" value={net && net.totalPeers != null ? net.totalPeers : 0} />
        <MetricCard label="Connected" value={net && net.connected != null ? net.connected : 0} />
        <MetricCard label="Degraded" value={net && net.degraded != null ? net.degraded : 0} />
        <MetricCard label="Unreachable" value={net && net.unreachable != null ? net.unreachable : 0} />
        <MetricCard label="Avg Latency" value={net && net.avgLatencyMs != null ? net.avgLatencyMs : "-"} unit="ms" />
        <MetricCard label="Active Tasks" value={tasks && tasks.active != null ? tasks.active : 0} />
        <MetricCard label="Completed Tasks" value={tasks && tasks.completed != null ? tasks.completed : 0} />
      </div>

      <NetworkGraph selfId={data && data.nodeId} peers={peers || []} />

      {peers && peers.length > 0 && (
        <div className="peer-table-wrap">
          <table aria-label="Peer list">
            <thead>
              <tr>
                <th>Peer URL</th>
                <th>Status</th>
                <th>Latency (ms)</th>
                <th>Last Seen</th>
              </tr>
            </thead>
            <tbody>
              {peers.map((p) => (
                <tr key={p.id || p.url}>
                  <td className="truncate">{p.url}</td>
                  <td>
                    <span className="pill" style={{ background: STATUS_COLOR[p.status] || STATUS_COLOR.unknown }}>
                      {p.status}
                    </span>
                  </td>
                  <td>{p.latencyMs != null ? p.latencyMs : "-"}</td>
                  <td>{p.lastSeen ? new Date(p.lastSeen).toLocaleTimeString() : "-"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {(!peers || peers.length === 0) && !error && (
        <p className="state">No peers registered. Add peers via POST /peers/register on the keeper.</p>
      )}
    </section>
  );
}