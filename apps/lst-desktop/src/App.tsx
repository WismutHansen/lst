import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Logo from "./assets/logo.png";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function fetchList() {
    setGreetMsg(await invoke("get_lists"));
  }

  return (
    <div className="background">
      <main className="container">
        <div className="row">
          <a href="https://github.com/WismutHansen/lst" target="_blank">
          </a>
        </div>
        <form
          className="row"
          onSubmit={(e) => { e.preventDefault(); fetchList(); }}
        >
          <div className="searchbar">
            <input
              id="greet-input"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="/"
            />
            <img src={Logo} alt="lst icon" className="search-icon" />
          </div>

          {/* <button type="submit">Show Lists</button> */}
        </form>

        <p>{greetMsg}</p>
        <div className="statusbar">
          <p>"hello world"</p>
        </div>
      </main>
    </div>
  );
}

export default App;
