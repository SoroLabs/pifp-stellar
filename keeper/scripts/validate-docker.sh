#!/bin/bash
# Validation script for Docker deployment

set -e

echo "🔍 Validating PIFP Keeper Docker deployment..."

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed"
    exit 1
fi
echo "✅ Docker is installed"

# Check if Docker Compose is available
if ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose is not available"
    exit 1
fi
echo "✅ Docker Compose is available"

# Check if .env file exists
if [ ! -f ".env" ]; then
    echo "⚠️  .env file not found, creating from .env.example"
    cp .env.example .env
fi
echo "✅ .env file exists"

# Build the image
echo "🔨 Building Docker image..."
docker build -t pifp-keeper:latest . > /dev/null 2>&1
echo "✅ Docker image built successfully"

# Start the container
echo "🚀 Starting container..."
docker compose up -d keeper > /dev/null 2>&1
sleep 5

# Check if container is running
if ! docker compose ps keeper | grep -q "Up"; then
    echo "❌ Container failed to start"
    docker compose logs keeper
    exit 1
fi
echo "✅ Container is running"

# Test health endpoint
echo "🏥 Testing health endpoint..."
HEALTH_RESPONSE=$(curl -s http://localhost:3000/health)
if echo "$HEALTH_RESPONSE" | grep -q "healthy"; then
    echo "✅ Health check passed"
else
    echo "❌ Health check failed"
    echo "Response: $HEALTH_RESPONSE"
    docker compose down
    exit 1
fi

# Test metrics endpoint
echo "📊 Testing metrics endpoint..."
METRICS_RESPONSE=$(curl -s http://localhost:3000/metrics)
if echo "$METRICS_RESPONSE" | grep -q "totalTasks"; then
    echo "✅ Metrics endpoint working"
else
    echo "❌ Metrics endpoint failed"
    echo "Response: $METRICS_RESPONSE"
    docker compose down
    exit 1
fi

# Check data volume
echo "💾 Checking data volume..."
if docker compose exec keeper test -d /app/data; then
    echo "✅ Data volume mounted correctly"
else
    echo "❌ Data volume not mounted"
    docker compose down
    exit 1
fi

# Clean up
echo "🧹 Cleaning up..."
docker compose down > /dev/null 2>&1

echo ""
echo "✨ All validation checks passed!"
echo "   The Keeper is ready for deployment."
