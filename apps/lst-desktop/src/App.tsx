import { useState } from "react";
import Logo from "./assets/logo.png";
import "./App.css";
import { commands } from "./bindings";

interface ListsOk {
  status: "ok";
  data: string[];
}

interface ListsErr {
  status: "error";
  error: string;
}

type ListResponse = ListsOk | ListsErr;


function App() {
  const [lists, setLists] = useState<ListResponse | null>(null);
  const [query, setQuery] = useState("");

  async function fetchLists() {
    try {
      const result: ListResponse = await commands.getLists();
      setLists(result);
    } catch (err) {
      // Network / IPC-level failures (rare with Tauri)
      console.error("Failed to fetch lists:", err);
      setLists({ status: "error", error: String(err) });
    }
  }


  function renderList(result: ListsOk) {
    return (
      <div className="list-wrapper">
        {result.data
          .filter((item) =>
            item.toLowerCase().includes(query.trim().toLowerCase())
          )
          .map((item) => (
            <div key={item} className="list-item">
              {item}
            </div>
          ))}
      </div>
    );
  }

  function renderError(err: ListsErr) {
    return <p className="error">⚠️ {err.error}</p>;
  }


  return (
    <div className="background">
      <main className="container">
        <div className="row">
          <a href="https://github.com/WismutHansen/lst" target="_blank"></a>
        </div>

        <form
          className="row"
          onSubmit={(e) => {
            e.preventDefault();
            fetchLists();
          }}
        >
          <div className="searchbar">
            <input
              id="query"
              value={query}
              onChange={(e) => setQuery(e.currentTarget.value)}
              placeholder="/"
              spellCheck="false"
            />
            <img src={Logo} alt="lst icon" className="search-icon" />

            <button className="button" type="button" onClick={fetchLists}>
              L
            </button>
          </div>
        </form>


        {lists &&
          (lists.status === "ok" ? renderList(lists) : renderError(lists))}

        <div className="statusbar">
          <p>hello world</p>
        </div>
      </main>
    </div>
  );
}

export default App;
