import { loadTaskRegistry } from '../taskRegistry.js';

/**
 * Health check controller for monitoring deep system status.
 * Performs checks on:
 * 1. Database/Persistence: Verifies task registry loading.
 * 2. RPC connectivity: Pings the Soroban RPC endpoint.
 */
export const getHealth = async (req, res) => {
  const health = {
    status: 'healthy',
    uptime: process.uptime(),
    timestamp: new Date().toISOString(),
    services: {
      database: 'connected',
      rpc: 'connected'
    }
  };

  try {
    // 1. Database Check (Task Registry)
    // We verify if we can at least load the tasks, simulating a DB check
    const tasks = loadTaskRegistry();
    if (!tasks) {
      throw new Error('Database (Task Registry) unavailable');
    }

    // 2. RPC Check
    // Call getLatestLedger or just a ping to the Soroban RPC URL
    const rpcUrl = process.env.STELLAR_RPC_URL;
    if (!rpcUrl) {
      throw new Error('RPC Configuration (STELLAR_RPC_URL) missing');
    }

    const rpcResponse = await fetch(rpcUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method: 'getLatestLedger', // Generic Soroban RPC method for Stellar
        params: []
      })
    });

    if (!rpcResponse.ok) {
        throw new Error(`RPC Provider unreachable (Status: ${rpcResponse.status})`);
    }
    
    res.status(200).json(health);
  } catch (error) {
    console.error('🔴 Health check failed:', error.message);
    
    health.status = 'unhealthy';
    health.error = error.message;
    
    // Map specific errors to services
    if (error.message.includes('Database')) health.services.database = 'disconnected';
    if (error.message.includes('RPC')) health.services.rpc = 'disconnected';

    res.status(503).json(health);
  }
};
