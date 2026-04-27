import { jest } from "@jest/globals";

jest.mock("../taskRegistry.js", () => ({
  loadTaskRegistry: jest.fn(() => []),
  saveTaskRegistry: jest.fn(),
}));

jest.mock("../p2pSync.js", () => ({
  getPeers: jest.fn(() => []),
  getNodeId: jest.fn(() => "test-node"),
  registerPeer: jest.fn(),
}));

import { loadTaskRegistry } from "../taskRegistry.js";
import { getPeers, getNodeId, registerPeer } from "../p2pSync.js";
import { listPeers, getPeerStatus, addPeer } from "../controllers/peerController.js";

function makeRes() {
  const res = {
    _status: 200,
    _body: null,
    status(code) { this._status = code; return this; },
    json(body) { this._body = body; return this; },
  };
  return res;
}

beforeEach(() => {
  jest.clearAllMocks();
  getPeers.mockReturnValue([]);
  getNodeId.mockReturnValue("test-node");
  loadTaskRegistry.mockReturnValue([]);
});

describe("listPeers", () => {
  test("returns 200 with nodeId and peerCount", () => {
    const res = makeRes();
    listPeers({}, res);
    expect(res._body).toMatchObject({ nodeId: "test-node", peerCount: 0, peers: [] });
  });

  test("includes registered peers", () => {
    const fakePeer = { id: "p1", url: "http://peer1.local", status: "connected", latencyMs: 50, lastSeen: "2024-01-01T00:00:00Z" };
    getPeers.mockReturnValue([fakePeer]);
    const res = makeRes();
    listPeers({}, res);
    expect(res._body.peerCount).toBe(1);
    expect(res._body.peers[0].url).toBe("http://peer1.local");
  });
});

describe("getPeerStatus", () => {
  test("returns network summary with correct counts", () => {
    getPeers.mockReturnValue([
      { status: "connected", latencyMs: 20 },
      { status: "degraded", latencyMs: 3000 },
      { status: "unreachable", latencyMs: 5000 },
    ]);
    loadTaskRegistry.mockReturnValue([
      { status: "active" },
      { status: "completed" },
    ]);
    const res = makeRes();
    getPeerStatus({}, res);
    expect(res._body.network.totalPeers).toBe(3);
    expect(res._body.network.connected).toBe(1);
    expect(res._body.network.degraded).toBe(1);
    expect(res._body.network.unreachable).toBe(1);
    expect(res._body.tasks.active).toBe(1);
    expect(res._body.tasks.completed).toBe(1);
  });

  test("avgLatencyMs is null when no peers have latency data", () => {
    getPeers.mockReturnValue([{ status: "unknown", latencyMs: null }]);
    const res = makeRes();
    getPeerStatus({}, res);
    expect(res._body.network.avgLatencyMs).toBeNull();
  });
});

describe("addPeer", () => {
  test("returns 201 for valid http URL", () => {
    registerPeer.mockReturnValue({ id: "p1", url: "http://peer1.local" });
    const res = makeRes();
    addPeer({ body: { url: "http://peer1.local" } }, res);
    expect(res._status).toBe(201);
    expect(res._body.peer.url).toBe("http://peer1.local");
  });

  test("returns 400 for missing url", () => {
    const res = makeRes();
    addPeer({ body: {} }, res);
    expect(res._status).toBe(400);
  });

  test("returns 400 for malformed url", () => {
    const res = makeRes();
    addPeer({ body: { url: "not-a-url" } }, res);
    expect(res._status).toBe(400);
  });

  test("returns 400 for non-http protocol (SSRF guard)", () => {
    const res = makeRes();
    addPeer({ body: { url: "ftp://peer.local" } }, res);
    expect(res._status).toBe(400);
    expect(res._body.error).toMatch(/http/i);
  });
});
