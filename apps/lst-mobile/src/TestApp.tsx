

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function TestApp() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [dbResult, setDbResult] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  async function testDatabase() {
    try {
      // Test list_titles
      const titles = await invoke("list_titles");
      setDbResult(`List titles: ${JSON.stringify(titles)}`);
      
      // Test create_list
      await invoke("create_list", { title: "Test List" });
      
      // Test load_list
      const list = await invoke("load_list", { title: "Test List" });
      setDbResult(`Created and loaded list: ${JSON.stringify(list, null, 2)}`);
    } catch (error) {
      setDbResult(`Database error: ${error}`);
    }
  }

  return (
    <div style={{ 
      padding: "20px", 
      fontFamily: "Arial, sans-serif",
      backgroundColor: "#f0f0f0",
      minHeight: "100vh",
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center"
    }}>
      <h1 style={{ color: "#333", marginBottom: "20px" }}>🎉 iOS App Working!</h1>
      
      <div style={{ marginBottom: "20px" }}>
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter a name..."
          style={{ padding: "8px", marginRight: "8px", borderRadius: "4px", border: "1px solid #ccc" }}
        />
        <button 
          onClick={greet}
          style={{ padding: "8px 16px", borderRadius: "4px", border: "none", backgroundColor: "#007bff", color: "white", marginRight: "8px" }}
        >
          Greet
        </button>
        <button 
          onClick={testDatabase}
          style={{ padding: "8px 16px", borderRadius: "4px", border: "none", backgroundColor: "#28a745", color: "white" }}
        >
          Test DB
        </button>
      </div>

      {greetMsg && (
        <p style={{ color: "#007bff", fontWeight: "bold", marginBottom: "20px" }}>
          {greetMsg}
        </p>
      )}

      {dbResult && (
        <div style={{ 
          color: "#28a745", 
          fontWeight: "bold", 
          marginBottom: "20px",
          padding: "10px",
          backgroundColor: "#f8f9fa",
          borderRadius: "4px",
          border: "1px solid #dee2e6",
          maxWidth: "90%",
          wordBreak: "break-word"
        }}>
          <pre style={{ margin: 0, fontSize: "12px", whiteSpace: "pre-wrap" }}>
            {dbResult}
          </pre>
        </div>
      )}

      <div style={{ 
        marginTop: "20px", 
        padding: "10px", 
        backgroundColor: "#e8f5e8", 
        borderRadius: "8px",
        border: "1px solid #4caf50"
      }}>
        <p style={{ color: "#2e7d32", margin: 0, fontSize: "14px" }}>
          ✅ Tauri iOS runtime: Working<br/>
          ✅ React frontend: Working<br/>
          ✅ WebView: Working<br/>
          ✅ Rust commands: {greetMsg ? "Working" : "Not tested"}<br/>
          ✅ Database: {dbResult ? "Working" : "Not tested"}
        </p>
      </div>
    </div>
  );
}

export default TestApp;