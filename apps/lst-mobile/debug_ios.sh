#!/bin/bash

echo "🔍 Starting iOS debugging session..."
echo "📱 Make sure your iOS device is connected and trusted"

# Create logs directory
mkdir -p logs

# Start capturing device logs in background
echo "📋 Starting device log capture..."
idevicesyslog | grep -E "(lst-mobile|crash|error|exception)" > logs/ios_device.log &
LOG_PID=$!

echo "🏗️ Building and deploying app..."
echo "   Press Ctrl+C after the app crashes to stop log capture"
echo ""

# Build and run the app on iOS device
bun tauri ios dev --config src-tauri/tauri.ios.conf.json --features mobile

# Stop log capture
echo ""
echo "🛑 Stopping log capture..."
kill $LOG_PID 2>/dev/null

echo "📋 Device logs saved to logs/ios_device.log"
echo "🔍 Checking for crash information..."

if [ -f logs/ios_device.log ]; then
    echo ""
    echo "=== RECENT CRASH/ERROR LOGS ==="
    tail -50 logs/ios_device.log
else
    echo "❌ No device logs captured"
fi