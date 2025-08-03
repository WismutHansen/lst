import React, { useState, useEffect } from "react";
import { commands } from "./bindings";

// Test version that adds Tauri API calls
export default function App() {
  const [lists, setLists] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadLists = async () => {
      try {
        console.log("🔄 Attempting to load lists...");
        const res = await commands.getLists();
        if (res.status === "ok") {
          console.log("✅ Successfully loaded lists:", res.data);
          setLists(res.data);
        } else {
          console.error("❌ Failed to load lists:", res.error);
          setError(res.error);
        }
      } catch (err) {
        console.error("💥 Exception loading lists:", err);
        setError(String(err));
      } finally {
        setLoading(false);
      }
    };

    loadLists();
  }, []);

  return (
    <div className="flex min-h-screen bg-white text-black p-4">
      <div className="flex flex-col w-full">
        <h1 className="text-2xl font-bold mb-4">lst Mobile - Tauri API Test</h1>
        
        {loading && (
          <div className="text-blue-600">Loading lists...</div>
        )}
        
        {error && (
          <div className="text-red-600 bg-red-50 p-3 rounded mb-4">
            Error: {error}
          </div>
        )}
        
        {!loading && !error && (
          <div>
            <h2 className="text-lg font-semibold mb-2">Lists ({lists.length}):</h2>
            <ul className="list-disc list-inside">
              {lists.map((list, index) => (
                <li key={index} className="text-sm">{list}</li>
              ))}
            </ul>
          </div>
        )}
        
        <div className="mt-4 p-4 border border-gray-300 rounded">
          <p className="text-sm text-gray-600">
            This version tests if the crash is caused by Tauri API calls.
          </p>
        </div>
      </div>
    </div>
  );
}