import { useState, useRef, useEffect, useMemo } from "react";
import Logo from "./assets/logo.png";
import "./App.css";
import { commands, type List, type ListItem } from "./bindings";
import { CommandPalette, PaletteCommand } from "./components/CommandPalette";

interface ListNode {
  name: string;
  path: string;
  isList: boolean;
  children: ListNode[];
}

function buildListTree(paths: string[]): ListNode[] {
  const root: Record<string, any> = {};
  for (const p of paths) {
    const parts = p.split("/");
    let node = root;
    let prefix = "";
    parts.forEach((part, idx) => {
      prefix = prefix ? `${prefix}/${part}` : part;
      if (!node[part]) {
        node[part] = { name: part, path: prefix, isList: false, children: {} };
      }
      if (idx === parts.length - 1) {
        node[part].isList = true;
      }
      node = node[part].children;
    });
  }
  const convert = (obj: Record<string, any>): ListNode[] =>
    Object.values(obj)
      .map((n: any) => ({
        name: n.name,
        path: n.path,
        isList: n.isList,
        children: convert(n.children),
      }))
      .sort((a: ListNode, b: ListNode) => a.name.localeCompare(b.name));
  return convert(root);
}

function App() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [lists, setLists] = useState<string[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [currentList, setCurrentList] = useState<List | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentName, setCurrentName] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newListName, setNewListName] = useState("");
  const [newItem, setNewItem] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [editingAnchor, setEditingAnchor] = useState<string | null>(null);
  const [editText, setEditText] = useState("");

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
      setCurrentName(name);
      setShowSuggestions(false);
      setQuery("");
    } else {
      setError(res.error);
    }
  }

  async function createNewList(e: React.FormEvent) {
    e.preventDefault();
    if (!newListName.trim()) return;
    const res = await commands.createList(newListName.trim());
    if (res.status === "ok") {
      await fetchLists();
      loadList(res.data.title);
      setNewListName("");
      setCreating(false);
    } else {
      setError(res.error);
    }
  }

  async function quickAddItem(e: React.FormEvent) {
    e.preventDefault();
    if (!currentName || !newItem.trim()) return;
    const res = await commands.addItem(currentName, newItem.trim());
    if (res.status === "ok") {
      setCurrentList(res.data);
      setNewItem("");
    } else {
      setError(res.error);
    }
  }

  const filtered = useMemo(() => {
    if (query === "*") return lists;
    return lists.filter((l) => l.toLowerCase().includes(query.toLowerCase()));
  }, [query, lists]);

  const listTree = useMemo(() => buildListTree(lists), [lists]);

  const paletteCommands = useMemo<PaletteCommand[]>(
    () => [
      {
        label: "New List",
        action: () => setCreating(true),
      },
      ...lists.map((l) => ({
        label: `Open ${l}`,
        action: () => loadList(l),
      })),
    ],
    [lists]
  );

  useEffect(() => {
    fetchLists();
  }, []);

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

  useEffect(() => {
    function handlePalette(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "p") {
        e.preventDefault();
        setPaletteOpen((o) => !o);
      }
    }
    window.addEventListener("keydown", handlePalette);
    return () => window.removeEventListener("keydown", handlePalette);
  }, []);

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
    async function toggle(anchor: string) {
      if (!currentName) return;
      const res = await commands.toggleItem(currentName, anchor);
      if (res.status === "ok") {
        setCurrentList(res.data);
      } else {
        setError(res.error);
      }
    }

    function startEdit(item: ListItem) {
      setEditingAnchor(item.anchor);
      setEditText(item.text);
    }

    async function saveEdit(anchor: string) {
      if (!currentName) return;
      const res = await commands.editItem(currentName, anchor, editText);
      if (res.status === "ok") {
        setCurrentList(res.data);
        setEditingAnchor(null);
        setEditText("");
      } else {
        setError(res.error);
      }
    }

    return (
      <div className="list-wrapper">
        <h2>{currentList.title}</h2>
        {currentList.items.map((it) =>
          editingAnchor === it.anchor ? (
            <form
              key={it.anchor}
              className="list-item list-entry"
              onSubmit={(e) => {
                e.preventDefault();
                saveEdit(it.anchor);
              }}
            >
              <input
                className="edit-input"
                value={editText}
                onChange={(e) => setEditText(e.currentTarget.value)}
                onBlur={() => saveEdit(it.anchor)}
                autoFocus
              />
            </form>
          ) : (
            <label key={it.anchor} className="list-item list-entry">
              <input
                type="checkbox"
                checked={it.status === "Done"}
                onChange={() => toggle(it.anchor)}
              />
              <span onDoubleClick={() => startEdit(it)}>{it.text}</span>
              <button
                type="button"
                className="edit-btn"
                onClick={() => startEdit(it)}
              >
                Edit
              </button>
            </label>
          )
        )}
        <form className="add-item-form" onSubmit={quickAddItem}>
          <input
            type="text"
            value={newItem}
            onChange={(e) => setNewItem(e.currentTarget.value)}
            placeholder="Add item"
          />
          <button type="submit">Add</button>
        </form>
      </div>
    );
  }

  function renderSidebar() {
    const renderNodes = (nodes: ListNode[], depth = 0): JSX.Element[] =>
      nodes.flatMap((n) => [
        <div
          key={n.path}
          className={
            "sidebar-item" + (n.isList && n.path === currentName ? " selected" : "")
          }
          style={{ paddingLeft: depth * 12 }}
          onClick={() => n.isList && loadList(n.path)}
        >
          {n.name}
        </div>,
        ...renderNodes(n.children, depth + 1),
      ]);

    return (
      <div className="sidebar">
        <h3>Lists</h3>
        <button onClick={() => setCreating((c) => !c)}>+ New List</button>
        {creating && (
          <form className="new-list-form" onSubmit={createNewList}>
            <input
              type="text"
              value={newListName}
              onChange={(e) => setNewListName(e.currentTarget.value)}
              placeholder="List name"
            />
            <button type="submit">Create</button>
          </form>
        )}
        <div className="sidebar-items">{renderNodes(listTree)}</div>
      </div>
    );
  }

  return (
    <div className="background">
      <div className="layout">
        {renderSidebar()}
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
        <CommandPalette
          open={paletteOpen}
          onClose={() => setPaletteOpen(false)}
          commands={paletteCommands}
        />
      </main>
      </div>
    </div>
  );
}

export default App;
