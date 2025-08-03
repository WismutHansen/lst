import React, { useState, useEffect } from "react";
import { commands } from "./bindings";

// Test version that adds theme functionality
export default function App() {
  const [lists, setLists] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [themeLoading, setThemeLoading] = useState(true);
  const [themeError, setThemeError] = useState<string | null>(null);

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

  useEffect(() => {
    const loadTheme = async () => {
      try {
        console.log("🎨 Attempting to load theme...");
        const result = await commands.getCurrentTheme();
        if (result.status === "ok") {
          console.log("✅ Successfully loaded theme:", result.data.name);
          // Apply theme to DOM
          const root = document.documentElement;
          const cssVariables = result.data.css_variables;
          const lines = cssVariables.split("\n");
          
          for (const line of lines) {
            const trimmed = line.trim();
            if (trimmed.startsWith("--") && trimmed.includes(":")) {
              const colonIndex = trimmed.indexOf(":");
              const property = trimmed.substring(0, colonIndex).trim();
              const value = trimmed.substring(colonIndex + 1).replace(";", "").trim();
              root.style.setProperty(property, value);
            }
          }
          
          root.classList.remove("light", "dark");
          root.classList.add(result.data.scheme);
        } else {
          console.error("❌ Failed to load theme:", result.error);
          setThemeError(result.error);
        }
      } catch (err) {
        console.error("💥 Exception loading theme:", err);
        setThemeError(String(err));
      } finally {
        setThemeLoading(false);
      }
    };

    loadTheme();
  }, []);

  return (
    <div className="flex min-h-screen p-4" style={{ backgroundColor: "var(--background, white)", color: "var(--foreground, black)" }}>
      <div className="flex flex-col w-full">
        <h1 className="text-2xl font-bold mb-4">lst Mobile - Theme Test</h1>
        
        {(loading || themeLoading) && (
          <div className="text-blue-600">Loading...</div>
        )}
        
        {error && (
          <div className="text-red-600 bg-red-50 p-3 rounded mb-4">
            Lists Error: {error}
          </div>
        )}
        
        {themeError && (
          <div className="text-red-600 bg-red-50 p-3 rounded mb-4">
            Theme Error: {themeError}
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
        
        <div className="mt-4 p-4 border rounded" style={{ borderColor: "var(--border, #ccc)" }}>
          <p className="text-sm" style={{ color: "var(--muted-foreground, #666)" }}>
            This version tests if the crash is caused by theme loading and DOM manipulation.
          </p>
        </div>
      </div>
    </div>
  );
}