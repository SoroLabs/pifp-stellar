# PIFP Keeper - Automation Bot

The Keeper is an off-chain automation service that monitors the PIFP protocol smart contracts on Stellar/Soroban and triggers scheduled tasks automatically.

## Overview

The Keeper bot:
- Monitors contract events for scheduled tasks
- Maintains a persistent task registry
- Triggers contract functions at the appropriate time
- Exposes health check and metrics endpoints

## Quick Start

### Prerequisites

- Node.js 20+ (for local development)
- Docker and Docker Compose (for containerized deployment)

### Local Development

1. Install dependencies:
```bash
npm install
```

2. Configure environment:
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. Run the keeper:
```bash
npm start
```

For development with auto-reload:
```bash
npm run dev
```

## Docker Deployment

The Keeper is designed to run as a Docker container for production deployments.

### Build the Docker Image

```bash
npm run docker:build
```

Or manually:
```bash
docker build -t pifp-keeper:latest .
```

### Run with Docker Compose

1. Configure environment:
```bash
cp .env.example .env
# Edit .env with your Stellar network configuration
```

2. Start the keeper:
```bash
docker compose up -d keeper
```

3. View logs:
```bash
docker compose logs -f keeper
```

4. Stop the keeper:
```bash
docker compose down
```

### Run from Repository Root

From the repository root directory:

```bash
docker compose up -d keeper
```

This will:
- Build the keeper image
- Start the container
- Mount `./keeper/data` for persistent task storage
- Load environment variables from `./keeper/.env`
- Expose port 3000 for health checks and metrics
- Automatically restart unless stopped manually

## Configuration

All configuration is done via environment variables in the `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | `3000` |
| `HOST` | HTTP server host | `0.0.0.0` |
| `POLL_INTERVAL_MS` | Task check interval in milliseconds | `30000` |
| `STELLAR_NETWORK` | Stellar network (testnet/mainnet) | `testnet` |
| `STELLAR_RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `CONTRACT_ADDRESS` | PIFP protocol contract address | - |
| `KEEPER_SECRET_KEY` | Keeper wallet secret key | - |
| `LOG_LEVEL` | Logging level | `info` |

## API Endpoints

### Health Check

```bash
GET /health
```

Returns the keeper's health status:

```json
{
  "status": "healthy",
  "uptime": 123.45,
  "timestamp": "2024-01-01T00:00:00.000Z"
}
```

### Metrics

```bash
GET /metrics
```

Returns operational metrics:

```json
{
  "totalTasks": 10,
  "activeTasks": 3,
  "completedTasks": 7,
  "uptime": 123.45
}
```

## Data Persistence

The keeper stores its task registry in `./data/tasks.json`. This file is automatically created and maintained by the keeper.

When running in Docker, this directory is mounted as a volume to ensure data persists across container restarts:

```yaml
volumes:
  - ./keeper/data:/app/data
```

## Architecture

The keeper consists of three main components:

1. **HTTP Server** (`src/index.js`): Exposes health check and metrics endpoints
2. **Task Registry** (`src/taskRegistry.js`): Manages persistent task storage
3. **Monitor** (`src/monitor.js`): Polls for tasks and triggers contract calls

## Security

- The container runs as a non-root user (`nodejs:nodejs`)
- Sensitive configuration is loaded from environment variables
- The `.env` file is excluded from the Docker image via `.dockerignore`
- Health checks ensure the keeper is responsive

## Deployment Options

### Cloud VM / VPS

Deploy to any cloud provider (AWS EC2, DigitalOcean, Linode, etc.):

```bash
# SSH into your VM
ssh user@your-server

# Clone the repository
git clone <repo-url>
cd pifp-stellar

# Configure the keeper
cd keeper
cp .env.example .env
nano .env  # Edit configuration

# Start with Docker Compose
cd ..
docker compose up -d keeper
```

### Container Orchestrators

The keeper can be deployed to Kubernetes, Docker Swarm, or other orchestrators:

**Kubernetes Example:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pifp-keeper
spec:
  replicas: 1
  selector:
    matchLabels:
      app: pifp-keeper
  template:
    metadata:
      labels:
        app: pifp-keeper
    spec:
      containers:
      - name: keeper
        image: pifp-keeper:latest
        ports:
        - containerPort: 3000
        envFrom:
        - secretRef:
            name: keeper-secrets
        volumeMounts:
        - name: data
          mountPath: /app/data
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 30
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: keeper-data
```

## Monitoring

The keeper exposes Prometheus-compatible metrics at `/metrics`. You can integrate with monitoring systems:

```bash
# Check health
curl http://localhost:3000/health

# Get metrics
curl http://localhost:3000/metrics
```

## Troubleshooting

### Container won't start

Check logs:
```bash
docker compose logs keeper
```

### Health check failing

Verify the keeper is listening:
```bash
docker compose exec keeper wget -O- http://localhost:3000/health
```

### Tasks not being processed

1. Check the task registry: `./keeper/data/tasks.json`
2. Verify Stellar network connectivity
3. Check keeper logs for errors

## Development

### Project Structure

```
keeper/
├── src/
│   ├── index.js          # Main entry point
│   ├── taskRegistry.js   # Task persistence
│   └── monitor.js        # Task monitoring
├── data/                 # Task registry storage (gitignored)
├── Dockerfile            # Multi-stage Docker build
├── .dockerignore         # Docker build exclusions
├── .env.example          # Environment template
├── package.json          # Node.js dependencies
└── README.md            # This file
```

### Adding Features

To extend the keeper:

1. Add new modules in `src/`
2. Update `src/index.js` to integrate
3. Add configuration to `.env.example`
4. Update this README

## License

MIT
