import { loadTaskRegistry, saveTaskRegistry } from './taskRegistry.js';

const POLL_INTERVAL = parseInt(process.env.POLL_INTERVAL_MS || '30000', 10);

export function startMonitoring() {
  console.log(`🔍 Starting task monitoring (interval: ${POLL_INTERVAL}ms)`);
  
  // Initial check
  checkTasks();
  
  // Periodic checks
  setInterval(checkTasks, POLL_INTERVAL);
}

function checkTasks() {
  const tasks = loadTaskRegistry();
  const activeTasks = tasks.filter(t => t.status === 'active');
  
  if (activeTasks.length > 0) {
    console.log(`⏰ Checking ${activeTasks.length} active tasks...`);
    // TODO: Implement actual task checking logic
    // This would involve querying the Stellar network for contract events
  }
}
