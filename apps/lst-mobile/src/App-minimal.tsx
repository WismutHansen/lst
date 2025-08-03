import React from "react";

// Minimal test component to isolate iOS crash
export default function App() {
  return (
    <div className="flex min-h-screen bg-white text-black p-4">
      <div className="flex flex-col items-center justify-center w-full">
        <h1 className="text-2xl font-bold mb-4">lst Mobile - Minimal Test</h1>
        <p className="text-lg">If you can see this, the basic React app works on iOS.</p>
        <div className="mt-4 p-4 border border-gray-300 rounded">
          <p className="text-sm text-gray-600">
            This is a minimal version to test if the crash is caused by:
          </p>
          <ul className="mt-2 text-sm text-gray-600 list-disc list-inside">
            <li>Heavy initialization code</li>
            <li>Tauri API calls</li>
            <li>Event listeners</li>
            <li>DOM manipulation</li>
            <li>Complex hooks</li>
          </ul>
        </div>
      </div>
    </div>
  );
}