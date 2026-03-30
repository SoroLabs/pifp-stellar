import express from 'express';
import { getHealth } from '../controllers/healthController.js';

const router = express.Router();

/**
 * Standard GET /health endpoint for monitoring.
 * Registered in src/index.js.
 */
router.get('/', getHealth);

export default router;
