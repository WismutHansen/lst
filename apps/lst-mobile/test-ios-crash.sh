#!/bin/bash

# Script to help test different versions of App.tsx to isolate iOS crash

echo "🔍 iOS Crash Testing Script for lst-mobile"
echo "=========================================="
echo ""

# Backup original App.tsx
if [ ! -f "src/App.tsx.backup" ]; then
    echo "📦 Backing up original App.tsx..."
    cp src/App.tsx src/App.tsx.backup
fi

echo "Available test versions:"
echo "1. Minimal (no Tauri, no complex logic)"
echo "2. Tauri API test (basic API calls)"
echo "3. Theme test (API calls + DOM manipulation)"
echo "4. Restore original"
echo ""

read -p "Which version would you like to test? (1-4): " choice

case $choice in
    1)
        echo "🧪 Switching to minimal test version..."
        cp src/App-minimal.tsx src/App.tsx
        ;;
    2)
        echo "🧪 Switching to Tauri API test version..."
        cp src/App-test-tauri.tsx src/App.tsx
        ;;
    3)
        echo "🧪 Switching to theme test version..."
        cp src/App-test-theme.tsx src/App.tsx
        ;;
    4)
        echo "🔄 Restoring original App.tsx..."
        cp src/App.tsx.backup src/App.tsx
        ;;
    *)
        echo "❌ Invalid choice. Exiting."
        exit 1
        ;;
esac

echo "✅ App.tsx updated. Now run:"
echo "   bun tauri build"
echo "   Then test on your iPhone."
echo ""
echo "💡 Testing strategy:"
echo "   - Start with version 1 (minimal)"
echo "   - If it works, try version 2"
echo "   - If version 2 crashes, the issue is with Tauri API calls"
echo "   - If version 2 works but 3 crashes, the issue is with theme/DOM manipulation"
echo "   - Continue until you find the breaking point"