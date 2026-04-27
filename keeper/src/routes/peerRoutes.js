import express from "express";
import { listPeers, getPeerStatus, addPeer } from "../controllers/peerController.js";
import { handleIncomingGossip } from "../p2pSync.js";

const router = express.Router();

router.get("/", listPeers);
router.get("/status", getPeerStatus);
router.post("/register", addPeer);

router.post("/gossip", (req, res) => {
  const result = handleIncomingGossip(req.body);
  res.json(result);
});

export default router;
