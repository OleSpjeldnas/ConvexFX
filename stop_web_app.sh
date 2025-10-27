#!/bin/bash
# ConvexFX Delta Web Application Stop Script

cd "$(dirname "$0")"

if [ -f logs/web_app.pid ]; then
    PID=$(cat logs/web_app.pid)
    echo "🛑 Stopping web app (PID: $PID)..."
    kill $PID 2>/dev/null || true
    rm logs/web_app.pid
    echo "✅ Server stopped"
else
    echo "⚠️  No PID file found. Killing any running web_app processes..."
    pkill -f "target/debug/web_app" || echo "No processes found"
fi

# Also kill any cargo processes running the web_app
pkill -f "cargo.*web_app" 2>/dev/null || true

echo "Done"

