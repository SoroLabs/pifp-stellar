import express from 'express';
import dotenv from 'dotenv';
import * as Sentry from '@sentry/node';
import * as Tracing from '@sentry/tracing';
import { loadTaskRegistry, saveTaskRegistry } from './taskRegistry.js';
import { startMonitoring } from './monitor.js';
import healthRoutes from './routes/healthRoutes.js';
import peerRoutes from './routes/peerRoutes.js';
import { initP2P, startGossipLoop } from './p2pSync.js';

dotenv.config();

Sentry.init({
  dsn: process.env.SENTRY_DSN || '',
  tracesSampleRate: 1.0,
  environment: process.env.NODE_ENV || 'development',
  beforeSend(event) {
    if (event.request?.headers) {
      const sensitiveHeaders = ['authorization', 'cookie', 'set-cookie', 'x-api-key'];
      sensitiveHeaders.forEach((name) => {
        if (event.request.headers[name]) {
          event.request.headers[name] = '[Filtered]';
        }
      });
    }

    if (event.user) {
      event.user.ip_address = '[Filtered]';
      event.user.email = event.user.email ? '[Filtered]' : undefined;
    }

    return event;
  },
});

process.on('unhandledRejection', (reason) => {
  Sentry.captureException(reason);
});

process.on('uncaughtException', (error) => {
  Sentry.captureException(error);
  console.error('Uncaught exception:', error);
  process.exit(1);
});

const app = express();
const PORT = process.env.PORT || 3000;
const HOST = process.env.HOST || '0.0.0.0';

// Global middleware
app.use(express.json());

// Monitoring & Health Endpoints
app.use('/health', healthRoutes);

// P2P peer endpoints
app.use('/peers', peerRoutes);

// Metrics endpoint
app.get('/metrics', (req, res) => {
  const tasks = loadTaskRegistry();
  res.json({
    totalTasks: tasks.length,
    activeTasks: tasks.filter(t => t.status === 'active').length,
    completedTasks: tasks.filter(t => t.status === 'completed').length,
    uptime: process.uptime()
  });
});

// Start the keeper
async function start() {
  try {
    console.log('🚀 Starting PIFP Keeper...');
    
    // Load task registry
    const tasks = loadTaskRegistry();
    console.log(`📋 Loaded ${tasks.length} tasks from registry`);

    // Initialise P2P gossip mesh
    initP2P();

    // Start monitoring
    startMonitoring();

    // Start gossip loop
    startGossipLoop();
    
    // Start HTTP server
    app.listen(PORT, HOST, () => {
      console.log(`✅ Keeper HTTP server listening on ${HOST}:${PORT}`);
      console.log(`   Health check: http://${HOST}:${PORT}/health`);
      console.log(`   Peer status:  http://${HOST}:${PORT}/peers/status`);
      console.log(`   Metrics: http://${HOST}:${PORT}/metrics`);
    });
  } catch (error) {
    Sentry.captureException(error);
    console.error('❌ Failed to start keeper:', error);
    process.exit(1);
  }
}

start();
