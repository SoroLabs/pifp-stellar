import { getHealth } from '../controllers/healthController.js';
import { loadTaskRegistry } from '../taskRegistry.js';

// Mocking loadTaskRegistry and global fetch
jest.mock('../taskRegistry.js');
global.fetch = jest.fn();

describe('Health Check Controller', () => {
  let req, res;

  beforeEach(() => {
    req = {};
    res = {
      status: jest.fn().mockReturnThis(),
      json: jest.fn()
    };
    jest.clearAllMocks();
  });

  test('should return 200 OK when both DB and RPC are healthy', async () => {
    // Mock healthy State
    loadTaskRegistry.mockReturnValue([{ id: 'test-task' }]);
    global.fetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ result: { ledger_index: 100 } })
    });

    await getHealth(req, res);

    expect(res.status).toHaveBeenCalledWith(200);
    expect(res.json).toHaveBeenCalledWith(expect.objectContaining({
      status: 'healthy',
      services: {
        database: 'connected',
        rpc: 'connected'
      }
    }));
  });

  test('should return 503 Service Unavailable when RPC is down', async () => {
    // Mock healthy DB but failing RPC
    loadTaskRegistry.mockReturnValue([{ id: 'test-task' }]);
    global.fetch.mockResolvedValue({
      ok: false,
      status: 504
    });

    await getHealth(req, res);

    expect(res.status).toHaveBeenCalledWith(503);
    expect(res.json).toHaveBeenCalledWith(expect.objectContaining({
      status: 'unhealthy',
      services: expect.objectContaining({
        rpc: 'disconnected'
      })
    }));
  });

  test('should return 503 Service Unavailable when DB is down', async () => {
    // Mock failing DB
    loadTaskRegistry.mockImplementation(() => { throw new Error('Database (Task Registry) unavailable'); });

    await getHealth(req, res);

    expect(res.status).toHaveBeenCalledWith(503);
    expect(res.json).toHaveBeenCalledWith(expect.objectContaining({
      status: 'unhealthy',
      services: expect.objectContaining({
        database: 'disconnected'
      })
    }));
  });
});
