import { loadTaskRegistry } from './taskRegistry.js';

const RPC_URL = process.env.STELLAR_RPC_URL || 'https://soroban-testnet.stellar.org';

/**
 * Checks storage health by verifying the task registry is readable.
 * Acts as the "DB" heartbeat for this file-backed service.
 */
async function checkDb() {
  try {
    loadTaskRegistry();
    return { ok: true };
  } catch (error) {
    return { ok: false, error: error.message };
  }
}

/**
 * Checks RPC connectivity by fetching the latest ledger (block) from the
 * Stellar Soroban RPC endpoint — equivalent to getBlockNumber on EVM chains.
 */
async function checkRpc() {
  try {
    const response = await fetch(RPC_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'getLatestLedger', params: [] }),
      signal: AbortSignal.timeout(5000),
    });

    if (!response.ok) {
      return { ok: false, error: `RPC responded with HTTP ${response.status}` };
    }

    const data = await response.json();
    if (data.error) {
      return { ok: false, error: data.error.message };
    }

    return { ok: true };
  } catch (error) {
    return { ok: false, error: error.message };
  }
}

/**
 * Runs all health checks and returns a structured result.
 * @returns {{ status: 'UP'|'DOWN', db: string, rpc: string, error?: string }}
 */
export async function checkHealth() {
  const [db, rpc] = await Promise.all([checkDb(), checkRpc()]);

  const allOk = db.ok && rpc.ok;

  const result = {
    status: allOk ? 'UP' : 'DOWN',
    db: db.ok ? 'OK' : 'ERROR',
    rpc: rpc.ok ? 'OK' : 'ERROR',
  };

  if (!allOk) {
    const errors = [];
    if (!db.ok) errors.push(`db: ${db.error}`);
    if (!rpc.ok) errors.push(`rpc: ${rpc.error}`);
    result.error = errors.join('; ');
  }

  return result;
}
