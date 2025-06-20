import { useState, useRef, useEffect, useMemo } from "react";
import Logo from "./assets/logo.png";
import "./App.css";
import { commands, type List } from "./bindings";

function App() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [lists, setLists] = useState<string[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [currentList, setCurrentList] = useState<List | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function fetchLists() {
    const res = await commands.getLists();
    if (res.status === "ok") {
      setLists(res.data);
    } else {
      setError(res.error);
    }
  }

  async function loadList(name: string) {
    const res = await commands.getList(name);
    if (res.status === "ok") {
      setCurrentList(res.data);
      setShowSuggestions(false);
      setQuery("");
    } else {
      setError(res.error);
    }
  }

  const filtered = useMemo(() => {
    if (query === "*") return lists;
    return lists.filter((l) => l.toLowerCase().includes(query.toLowerCase()));
  }, [query, lists]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "/" && document.activeElement !== inputRef.current) {
        e.preventDefault();
        inputRef.current?.focus();
        setShowSuggestions(true);
      } else if (showSuggestions) {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setSelectedIndex((i) => (i + 1) % Math.max(filtered.length, 1));
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          setSelectedIndex((i) => (i - 1 + filtered.length) % Math.max(filtered.length, 1));
        } else if (e.key === "Enter") {
          e.preventDefault();
          if (filtered[selectedIndex]) {
            loadList(filtered[selectedIndex]);
          }
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [showSuggestions, filtered, selectedIndex]);

  function handleFocus() {
    fetchLists();
    setShowSuggestions(true);
  }

  function renderSuggestions() {
    if (!showSuggestions || filtered.length === 0) return null;
    return (
      <div className="list-wrapper">
        {filtered.map((item, idx) => (
          <div
            key={item}
            className={"list-item" + (idx === selectedIndex ? " selected" : "")}
            onMouseDown={() => loadList(item)}
          >
            {item}
          </div>
        ))}
      </div>
    );
  }

  function renderCurrentList() {
    if (!currentList) return null;
    return (
      <div className="list-wrapper">
        <h2>{currentList.title}</h2>
        {currentList.items.map((it) => (
          <div key={it.anchor} className="list-item">
            {it.status === "Done" ? "[x]" : "[ ]"} {it.text}
          </div>
        ))}
      </div>
    );
  }

  return (
    <div className="background">
      <main className="container">
        <div className="row">
          <a href="https://github.com/WismutHansen/lst" target="_blank"></a>
        </div>
        <form className="row" onSubmit={(e) => e.preventDefault()}>
          <div className="searchbar">
            <input
              id="query"
              ref={inputRef}
              value={query}
              onChange={(e) => setQuery(e.currentTarget.value)}
              onFocus={handleFocus}
              placeholder="/"
              spellCheck="false"
            />
            <img src={Logo} alt="lst icon" className="search-icon" />
          </div>
        </form>
        {renderSuggestions()}
        {renderCurrentList()}
        {error && <p className="error">⚠️ {error}</p>}
        <div className="statusbar">
          <p>hello world</p>
        </div>
      </main>
    </div>
  );
}

export default App;
