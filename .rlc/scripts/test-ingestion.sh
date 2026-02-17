#!/bin/bash
# Test event ingestion

echo "Testing event ingestion..."

# Check if observability stack is running
if docker ps | grep -q prometheus; then
    echo "✅ Prometheus is running"
else
    echo "⚠️  Prometheus not running (start with: docker-compose -f docker-compose.obs.yml up -d)"
fi

# Test metrics endpoint
if curl -s http://localhost:8080/metrics > /dev/null; then
    echo "✅ Metrics endpoint accessible"
else
    echo "⚠️  Metrics endpoint not accessible"
fi
