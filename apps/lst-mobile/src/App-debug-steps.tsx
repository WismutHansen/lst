import React, { useState, useEffect } from "react";

// Progressive loading test - each step adds more complexity
export default function App() {
  const [currentStep, setCurrentStep] = useState(0);
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const addLog = (message: string) => {
    const logMessage = `Step ${currentStep}: ${message}`;
    console.log(`📝 ${logMessage}`);
    setLogs(prev => [...prev, logMessage]);
  };

  const steps = [
    {
      name: "Basic React Mount",
      test: async () => {
        addLog("React component mounted successfully");
        await new Promise(resolve => setTimeout(resolve, 500));
      }
    },
    {
      name: "State Updates",
      test: async () => {
        addLog("Testing state updates");
        for (let i = 0; i < 3; i++) {
          await new Promise(resolve => setTimeout(resolve, 200));
          addLog(`State update ${i + 1}/3`);
        }
      }
    },
    {
      name: "DOM Manipulation",
      test: async () => {
        addLog("Testing DOM access");
        const testDiv = document.createElement('div');
        testDiv.textContent = 'Test element';
        document.body.appendChild(testDiv);
        await new Promise(resolve => setTimeout(resolve, 100));
        document.body.removeChild(testDiv);
        addLog("DOM manipulation successful");
      }
    },
    {
      name: "CSS Variables",
      test: async () => {
        addLog("Testing CSS variable manipulation");
        const root = document.documentElement;
        root.style.setProperty('--test-color', '#ff0000');
        await new Promise(resolve => setTimeout(resolve, 100));
        root.style.removeProperty('--test-color');
        addLog("CSS variables working");
      }
    },
    {
      name: "Local Storage",
      test: async () => {
        addLog("Testing localStorage");
        localStorage.setItem('test-key', 'test-value');
        const value = localStorage.getItem('test-key');
        localStorage.removeItem('test-key');
        if (value === 'test-value') {
          addLog("localStorage working");
        } else {
          throw new Error("localStorage test failed");
        }
      }
    },
    {
      name: "Async Operations",
      test: async () => {
        addLog("Testing async operations");
        const promises = Array.from({ length: 3 }, (_, i) => 
          new Promise(resolve => setTimeout(() => resolve(i), 100 * (i + 1)))
        );
        const results = await Promise.all(promises);
        addLog(`Async operations completed: ${results.join(', ')}`);
      }
    },
    {
      name: "Event Listeners",
      test: async () => {
        addLog("Testing event listeners");
        let eventFired = false;
        const handler = () => { eventFired = true; };
        
        window.addEventListener('resize', handler);
        await new Promise(resolve => setTimeout(resolve, 100));
        window.removeEventListener('resize', handler);
        
        addLog("Event listeners working");
      }
    },
    {
      name: "Complex State",
      test: async () => {
        addLog("Testing complex state management");
        const complexState = {
          arrays: [1, 2, 3],
          objects: { nested: { value: 'test' } },
          functions: () => 'test'
        };
        
        // Simulate complex state updates
        for (let i = 0; i < 5; i++) {
          complexState.arrays.push(i + 4);
          await new Promise(resolve => setTimeout(resolve, 50));
        }
        
        addLog("Complex state management working");
      }
    }
  ];

  useEffect(() => {
    const runTests = async () => {
      try {
        for (let i = 0; i < steps.length; i++) {
          setCurrentStep(i);
          addLog(`Starting: ${steps[i].name}`);
          
          await steps[i].test();
          
          addLog(`✅ Completed: ${steps[i].name}`);
          await new Promise(resolve => setTimeout(resolve, 300));
        }
        
        setCurrentStep(steps.length);
        addLog("🎉 All tests completed! The crash is likely in complex components.");
        
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`❌ Failed at step ${currentStep} (${steps[currentStep]?.name}): ${errorMessage}`);
        addLog(`❌ Test failed: ${errorMessage}`);
        console.error("Debug test error:", err);
      }
    };

    runTests();
  }, []);

  return (
    <div className="flex min-h-screen bg-white text-black p-4">
      <div className="flex flex-col w-full max-w-2xl mx-auto">
        <h1 className="text-3xl font-bold mb-6 text-center">
          lst-mobile Debug Test
        </h1>
        
        <div className="mb-6">
          <div className="flex justify-between items-center mb-2">
            <span className="text-lg font-semibold">
              Progress: {currentStep}/{steps.length}
            </span>
            <span className="text-sm text-gray-600">
              {currentStep < steps.length ? steps[currentStep]?.name : "Complete"}
            </span>
          </div>
          
          <div className="w-full bg-gray-200 rounded-full h-3">
            <div 
              className="bg-blue-600 h-3 rounded-full transition-all duration-500" 
              style={{ width: `${(currentStep / steps.length) * 100}%` }}
            />
          </div>
        </div>

        {error && (
          <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
            <strong>Error:</strong> {error}
          </div>
        )}

        <div className="bg-gray-50 border rounded-lg p-4 flex-1">
          <h2 className="font-semibold mb-3 text-lg">Test Log:</h2>
          <div className="space-y-1 max-h-96 overflow-y-auto">
            {logs.map((log, index) => (
              <div 
                key={index} 
                className={`text-sm font-mono p-2 rounded ${
                  log.includes('❌') ? 'bg-red-100 text-red-800' :
                  log.includes('✅') ? 'bg-green-100 text-green-800' :
                  log.includes('🎉') ? 'bg-blue-100 text-blue-800' :
                  'bg-white'
                }`}
              >
                {log}
              </div>
            ))}
          </div>
        </div>

        <div className="mt-4 p-4 bg-blue-50 rounded-lg">
          <h3 className="font-semibold mb-2">What This Test Shows:</h3>
          <ul className="text-sm space-y-1">
            <li>• If it crashes before step 3: Basic React/JS issue</li>
            <li>• If it crashes at step 4: CSS/DOM manipulation issue</li>
            <li>• If it crashes at step 7: Event listener issue</li>
            <li>• If it completes all steps: Issue is in lst-mobile components</li>
          </ul>
        </div>
      </div>
    </div>
  );
}