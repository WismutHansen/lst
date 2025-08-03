#!/bin/bash

echo "🔍 iOS Crash Debugging Script"
echo "============================="
echo ""

# Check if we have the necessary tools
if ! command -v xcrun &> /dev/null; then
    echo "❌ Xcode command line tools not found. Please install Xcode."
    exit 1
fi

echo "📱 Available iOS debugging methods:"
echo "1. View iOS device console logs (real-time)"
echo "2. Extract crash reports from device"
echo "3. Enable debug logging in Tauri config"
echo "4. Add JavaScript error handlers"
echo "5. Create minimal crash test app"
echo ""

read -p "Which method would you like to use? (1-5): " choice

case $choice in
    1)
        echo "🔄 Starting iOS device console monitoring..."
        echo "📋 Instructions:"
        echo "   1. Connect your iPhone via USB"
        echo "   2. Trust this computer on your iPhone"
        echo "   3. Launch lst-mobile on your iPhone"
        echo "   4. Watch for crash logs below"
        echo ""
        echo "Press Ctrl+C to stop monitoring"
        echo "================================"
        
        # Monitor iOS device logs
        xcrun devicectl list devices
        echo ""
        echo "🔍 Monitoring device logs (filtering for lst-mobile)..."
        xcrun devicectl log stream --device-id $(xcrun devicectl list devices | grep iPhone | head -1 | awk '{print $3}' | tr -d '()') --predicate 'process CONTAINS "lst"' 2>/dev/null || {
            echo "⚠️  If the above failed, try manually with:"
            echo "   xcrun devicectl log stream --device-id YOUR_DEVICE_ID"
            echo ""
            echo "📱 Available devices:"
            xcrun devicectl list devices
        }
        ;;
    2)
        echo "📄 Extracting crash reports..."
        echo "📋 Instructions:"
        echo "   1. Go to iPhone Settings > Privacy & Security > Analytics & Improvements > Analytics Data"
        echo "   2. Look for crash reports containing 'lst-mobile' or your app identifier"
        echo "   3. Share the crash report via AirDrop or email"
        echo ""
        echo "🔍 You can also check macOS crash reports at:"
        echo "   ~/Library/Logs/DiagnosticReports/"
        ls -la ~/Library/Logs/DiagnosticReports/ | grep -i lst 2>/dev/null || echo "   (No lst-related crash reports found)"
        ;;
    3)
        echo "🔧 Enabling debug logging in Tauri config..."
        
        # Backup original config
        cp src-tauri/tauri.conf.json src-tauri/tauri.conf.json.backup
        
        # Add debug configuration
        cat > src-tauri/tauri.conf.json.debug << 'EOF'
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "lst-mobile-debug",
  "version": "0.1.0",
  "identifier": "com.lst-mobile.debug",
  "build": {
    "beforeDevCommand": "bun run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "bun run build",
    "frontendDist": "../dist"
  },
  "app": {
    "macOSPrivateApi": false,
    "windows": [
      {
        "title": "lst-mobile-debug",
        "width": 800,
        "height": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null,
      "capabilities": [
        {
          "identifier": "mobile-permissions",
          "windows": ["*"],
          "permissions": [
            "core:default",
            "core:event:allow-listen",
            "core:event:default",
            "core:path:default",
            "core:app:default"
          ]
        }
      ]
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  },
  "plugins": {
    "log": {
      "level": "trace",
      "targets": [
        {
          "target": "LogTarget::Stdout"
        },
        {
          "target": "LogTarget::Webview"
        }
      ]
    }
  }
}
EOF
        
        mv src-tauri/tauri.conf.json.debug src-tauri/tauri.conf.json
        echo "✅ Debug logging enabled. Build and test the app."
        echo "📝 To restore original config: cp src-tauri/tauri.conf.json.backup src-tauri/tauri.conf.json"
        ;;
    4)
        echo "🔧 Adding JavaScript error handlers..."
        
        # Create error handling wrapper
        cat > src/App-with-error-handling.tsx << 'EOF'
import React, { ErrorInfo, ReactNode } from "react";
import { commands } from "./bindings";

// Error Boundary Component
class ErrorBoundary extends React.Component<
  { children: ReactNode },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("🚨 React Error Boundary caught error:", error);
    console.error("📍 Error Info:", errorInfo);
    
    // Try to log to Tauri backend if possible
    try {
      // You could add a Tauri command to log errors
      console.log("💾 Attempting to save error log...");
    } catch (e) {
      console.error("❌ Failed to log error to backend:", e);
    }
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex min-h-screen items-center justify-center bg-red-50 p-4">
          <div className="max-w-md text-center">
            <h1 className="text-2xl font-bold text-red-800 mb-4">App Crashed</h1>
            <p className="text-red-600 mb-4">
              The app encountered an error and crashed.
            </p>
            <details className="text-left bg-white p-4 rounded border">
              <summary className="cursor-pointer font-semibold">Error Details</summary>
              <pre className="mt-2 text-xs overflow-auto">
                {this.state.error?.toString()}
                {this.state.error?.stack}
              </pre>
            </details>
            <button
              onClick={() => window.location.reload()}
              className="mt-4 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
            >
              Reload App
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

// Global error handlers
window.addEventListener('error', (event) => {
  console.error('🚨 Global JavaScript Error:', event.error);
  console.error('📍 Error details:', {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
    stack: event.error?.stack
  });
});

window.addEventListener('unhandledrejection', (event) => {
  console.error('🚨 Unhandled Promise Rejection:', event.reason);
  console.error('📍 Promise:', event.promise);
});

// Import the original App component
import OriginalApp from "./App";

// Wrapped App with error boundary
export default function App() {
  return (
    <ErrorBoundary>
      <OriginalApp />
    </ErrorBoundary>
  );
}
EOF
        
        # Backup original main.tsx
        cp src/main.tsx src/main.tsx.backup
        
        echo "✅ Error handling added to src/App-with-error-handling.tsx"
        echo "📝 To use it, replace the import in src/main.tsx:"
        echo "   import App from \"./App-with-error-handling\";"
        ;;
    5)
        echo "🧪 Creating minimal crash test app..."
        
        cat > src/App-crash-test.tsx << 'EOF'
import React, { useState, useEffect } from "react";

export default function App() {
  const [step, setStep] = useState(0);
  const [logs, setLogs] = useState<string[]>([]);
  
  const addLog = (message: string) => {
    console.log(`📝 Step ${step}: ${message}`);
    setLogs(prev => [...prev, `Step ${step}: ${message}`]);
  };

  useEffect(() => {
    const runTest = async () => {
      try {
        addLog("App component mounted");
        setStep(1);
        
        await new Promise(resolve => setTimeout(resolve, 1000));
        addLog("1 second delay completed");
        setStep(2);
        
        // Test basic React state
        addLog("Testing React state updates");
        setStep(3);
        
        await new Promise(resolve => setTimeout(resolve, 1000));
        addLog("Basic React functionality working");
        setStep(4);
        
        // Test DOM manipulation
        addLog("Testing DOM access");
        const element = document.createElement('div');
        element.textContent = 'Test element';
        document.body.appendChild(element);
        document.body.removeChild(element);
        addLog("DOM manipulation successful");
        setStep(5);
        
        addLog("All basic tests passed - crash is likely in complex components");
        
      } catch (error) {
        addLog(`❌ Error at step ${step}: ${error}`);
        console.error("Crash test error:", error);
      }
    };
    
    runTest();
  }, []);

  return (
    <div className="flex min-h-screen bg-white text-black p-4">
      <div className="flex flex-col w-full">
        <h1 className="text-2xl font-bold mb-4">lst-mobile Crash Test</h1>
        <div className="mb-4">
          <div className="text-lg">Current Step: {step}</div>
          <div className="w-full bg-gray-200 rounded-full h-2.5 mt-2">
            <div 
              className="bg-blue-600 h-2.5 rounded-full transition-all duration-500" 
              style={{ width: `${(step / 5) * 100}%` }}
            ></div>
          </div>
        </div>
        
        <div className="bg-gray-100 p-4 rounded max-h-96 overflow-y-auto">
          <h2 className="font-semibold mb-2">Test Logs:</h2>
          {logs.map((log, index) => (
            <div key={index} className="text-sm font-mono mb-1">
              {log}
            </div>
          ))}
        </div>
        
        <div className="mt-4 text-sm text-gray-600">
          <p>If you can see this and the progress bar is moving, basic React is working.</p>
          <p>If the app crashes before step 5, the issue is in fundamental React/JS functionality.</p>
          <p>If it completes all steps, the crash is in the complex lst-mobile components.</p>
        </div>
      </div>
    </div>
  );
}
EOF
        
        echo "✅ Crash test app created: src/App-crash-test.tsx"
        echo "📝 To use it, update src/main.tsx to import from \"./App-crash-test\""
        ;;
    *)
        echo "❌ Invalid choice. Exiting."
        exit 1
        ;;
esac

echo ""
echo "💡 Additional debugging tips:"
echo "   - Check iPhone Settings > Privacy & Security > Analytics Data for crash reports"
echo "   - Use Safari Web Inspector to debug the webview (if it loads)"
echo "   - Try building with --debug flag: bun tauri build --debug"
echo "   - Check Xcode console when building/running"