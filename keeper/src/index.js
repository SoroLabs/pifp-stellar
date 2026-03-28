import express from 'express';
import dotenv from 'dotenv';
import { loadTaskRegistry, saveTaskRegistry } from './taskRegistry.js';
import { startMonitoring } from './monitor.js';
import { checkHealth } from './health.js';

dotenv.config();

const app = express();
const PORT = process.env.PORT || 3000;
const HOST = process.env.HOST || '0.0.0.0';

app.use(express.json());

// Health check endpoint
app.get('/health', async (req, res) => {
  const result = await checkHealth();
  const statusCode = result.status === 'UP' ? 200 : 503;
  res.status(statusCode).json(result);
});

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
    
    // Start monitoring
    startMonitoring();
    
    // Start HTTP server
    app.listen(PORT, HOST, () => {
      console.log(`✅ Keeper HTTP server listening on ${HOST}:${PORT}`);
      console.log(`   Health check: http://${HOST}:${PORT}/health`);
      console.log(`   Metrics: http://${HOST}:${PORT}/metrics`);
    });
  } catch (error) {
    console.error('❌ Failed to start keeper:', error);
    process.exit(1);
  }
}

start();
