import { jest } from "@jest/globals";

jest.mock("../taskRegistry.js");

import { loadTaskRegistry, saveTaskRegistry } from "../taskRegistry.js";
import {
  initP2P,
  registerPeer,
  getPeers,
  getNodeId,
  mergeRemoteState,
  handleIncomingGossip,
  gossipToPeer,
} from "../p2pSync.js";

beforeEach(() => {
  jest.clearAllMocks();
  loadTaskRegistry.mockReturnValue([]);
  initP2P({ nodeId: "test-node" });
});

describe("initP2P", () => {
  test("sets nodeId", () => {
    expect(getNodeId()).toBe("test-node");
  });

  test("clears peers on reinit", () => {
    registerPeer("http://peer1.local");
    initP2P({ nodeId: "test-node-2" });
    expect(getPeers()).toHaveLength(0);
  });
});

describe("registerPeer", () => {
  test("adds a new peer", () => {
    registerPeer("http://peer1.local");
    expect(getPeers()).toHaveLength(1);
    expect(getPeers()[0].url).toBe("http://peer1.local");
  });

  test("is idempotent for same URL", () => {
    registerPeer("http://peer1.local");
    registerPeer("http://peer1.local");
    expect(getPeers()).toHaveLength(1);
  });

  test("returns null for empty string", () => {
    expect(registerPeer("")).toBeNull();
  });

  test("returns null for null", () => {
    expect(registerPeer(null)).toBeNull();
  });
});

describe("mergeRemoteState", () => {
  test("adds unknown tasks from remote", () => {
    loadTaskRegistry.mockReturnValue([]);
    mergeRemoteState({ tasks: [{ id: "t1", status: "active", updatedAt: "2024-01-01T00:00:00Z" }] });
    expect(saveTaskRegistry).toHaveBeenCalledWith(
      expect.arrayContaining([expect.objectContaining({ id: "t1" })])
    );
  });

  test("applies newer remote over local", () => {
    loadTaskRegistry.mockReturnValue([
      { id: "t1", status: "active", updatedAt: "2024-01-01T00:00:00Z" },
    ]);
    mergeRemoteState({ tasks: [{ id: "t1", status: "completed", updatedAt: "2024-06-01T00:00:00Z" }] });
    expect(saveTaskRegistry).toHaveBeenCalledWith(
      expect.arrayContaining([expect.objectContaining({ id: "t1", status: "completed" })])
    );
  });

  test("keeps newer local over remote", () => {
    loadTaskRegistry.mockReturnValue([
      { id: "t1", status: "active", updatedAt: "2025-01-01T00:00:00Z" },
    ]);
    mergeRemoteState({ tasks: [{ id: "t1", status: "pending", updatedAt: "2024-01-01T00:00:00Z" }] });
    expect(saveTaskRegistry).not.toHaveBeenCalled();
  });

  test("no-op for empty tasks array", () => {
    mergeRemoteState({ tasks: [] });
    expect(saveTaskRegistry).not.toHaveBeenCalled();
  });

  test("no-op for null", () => {
    mergeRemoteState(null);
    expect(saveTaskRegistry).not.toHaveBeenCalled();
  });
});

describe("handleIncomingGossip", () => {
  test("merges remote state and returns local state", () => {
    loadTaskRegistry.mockReturnValue([{ id: "t2", status: "active" }]);
    const result = handleIncomingGossip({ nodeId: "remote-node", tasks: [{ id: "t2", status: "active" }] });
    expect(result).toHaveProperty("nodeId");
    expect(result).toHaveProperty("tasks");
    expect(result).toHaveProperty("timestamp");
  });
});

describe("gossipToPeer", () => {
  test("marks peer connected on 200 response", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: jest.fn().mockResolvedValue({ nodeId: "remote", tasks: [] }),
    });
    const peer = await gossipToPeer("http://peer1.local");
    expect(peer.status).toBe("connected");
  });

  test("marks peer unreachable on network error", async () => {
    global.fetch = jest.fn().mockRejectedValue(new Error("ECONNREFUSED"));
    const peer = await gossipToPeer("http://peer2.local");
    expect(peer.status).toBe("unreachable");
  });

  test("marks peer degraded on non-ok response", async () => {
    global.fetch = jest.fn().mockResolvedValue({ ok: false });
    const peer = await gossipToPeer("http://peer3.local");
    expect(peer.status).toBe("degraded");
  });
});
