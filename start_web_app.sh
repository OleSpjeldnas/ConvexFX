#!/bin/bash
# ConvexFX Delta Web Application Launcher
# This script starts the web application in the background

set -e

cd "$(dirname "$0")"

echo "🚀 Building and starting ConvexFX Delta Web Application..."
echo ""

# Build the application
cargo build --bin web_app --features runtime --quiet

# Start the server in the background
cargo run --bin web_app --features runtime > logs/web_app.log 2>&1 &
WEB_PID=$!

# Save the PID
mkdir -p logs
echo $WEB_PID > logs/web_app.pid

echo "⏳ Waiting for server to start..."
sleep 3

# Check if the server is responding
if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
    echo "✅ Server started successfully!"
    echo ""
    echo "📊 Web interface: http://localhost:8080"
    echo "📈 API metrics: http://localhost:8080/api/metrics"
    echo "💚 Health check: http://localhost:8080/api/health"
    echo ""
    echo "📝 Server PID: $WEB_PID (saved to logs/web_app.pid)"
    echo "📜 Logs: logs/web_app.log"
    echo ""
    echo "To stop the server, run: ./stop_web_app.sh"
else
    echo "❌ Server failed to start. Check logs/web_app.log for details."
    kill $WEB_PID 2>/dev/null || true
    exit 1
fi

