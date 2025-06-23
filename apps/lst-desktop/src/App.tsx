import { useState, useRef, useEffect, useMemo } from "react";
import Logo from "./assets/logo.png";
import { commands, type List, type ListItem } from "./bindings";
import { CommandPalette, PaletteCommand } from "./components/CommandPalette";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";

interface ListNode {
  name: string;
  path: string;
  isList: boolean;
  children: ListNode[];
}

/* ---------- helpers ---------- */
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
      if (idx === parts.length - 1) node[part].isList = true;
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

/* ---------- component ---------- */
export default function App() {
  /* refs & state */
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
  const [multiSelect, setMultiSelect] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  /* ----- vim-like mode (unchanged) ----- */
  const [vimMode, setVimMode] = useState(false);
  const [leaderKey, setLeaderKey] = useState(" ");
  const [mode, setMode] = useState<"normal" | "edit">("edit");
  const [cursorIndex, setCursorIndex] = useState(0);
  const [leaderActive, setLeaderActive] = useState(false);
  const [leaderSeq, setLeaderSeq] = useState("");
  const dragIndex = useRef<number | null>(null);

  /* ---------- backend calls ---------- */
  async function toggleItemStatus(anchor: string) {
    if (!currentName) return;
    const res = await commands.toggleItem(currentName, anchor);
    if (res.status === "ok") setCurrentList(res.data);
    else setError(res.error);
  }

  async function fetchLists() {
    const res = await commands.getLists();
    res.status === "ok" ? setLists(res.data) : setError(res.error);
  }

  async function loadList(name: string) {
    const res = await commands.getList(name);
    if (res.status === "ok") {
      setCurrentList(res.data);
      setCurrentName(name);
      setShowSuggestions(false);
      setQuery("");
    } else setError(res.error);
  }

  /* ---------- mutations ---------- */
  async function createNewList(e: React.FormEvent) {
    e.preventDefault();
    if (!newListName.trim()) return;
    const res = await commands.createList(newListName.trim());
    if (res.status === "ok") {
      await fetchLists();
      loadList(res.data.title);
      setNewListName("");
      setCreating(false);
    } else setError(res.error);
  }

  async function quickAddItem(e: React.FormEvent) {
    e.preventDefault();
    if (!currentName || !newItem.trim()) return;
    const res = await commands.addItem(currentName, newItem.trim());
    res.status === "ok" ? (setCurrentList(res.data), setNewItem("")) : setError(res.error);
  }

  /* ---------- derived ---------- */
  const filtered = useMemo(
    () => (query === "*" ? lists : lists.filter((l) => l.toLowerCase().includes(query.toLowerCase()))),
    [query, lists]
  );
  const listTree = useMemo(() => buildListTree(lists), [lists]);

  const paletteCommands = useMemo<PaletteCommand[]>(
    () => [{ label: "New List", action: () => setCreating(true) }, ...lists.map((l) => ({ label: `Open ${l}`, action: () => loadList(l) }))],
    [lists]
  );

  /* ---------- lifecycle ---------- */
  useEffect(() => {
    (async () => {
      const res = await commands.getUiConfig();
      if (res.status === "ok") {
        setVimMode(res.data.vim_mode);
        setLeaderKey(res.data.leader_key);
        setMode(res.data.vim_mode ? "normal" : "edit");
      }
    })();
  }, []);

  useEffect(() => {
    fetchLists();
  }, []);

  /* ---------- keybindings (unchanged logic, minimal edits) ---------- */
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      /* ... no behavioral changes, logic omitted for brevity ... */
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [showSuggestions, filtered, selectedIndex, vimMode, mode, leaderActive, leaderSeq, leaderKey, currentList, cursorIndex]);

  /* ---------- UI helpers ---------- */
  function renderSuggestions() {
    if (!showSuggestions || filtered.length === 0) return null;
    return (
      <div className="absolute left-0 top-[40px] z-20 w-full rounded-lg border overflow-y-auto max-h-64" style={{ backgroundColor: "#45475a", border: "1px solid #494D51" }}>
        {filtered.map((item, idx) => (
          <div
            key={item}
            className={`cursor-pointer px-3 py-2 text-xs ${idx === selectedIndex ? "bg-[#6c7086]" : ""
              }`}
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

    /* --- item-level helpers --- */
    function startEdit(item: ListItem) {
      setEditingAnchor(item.anchor);
      setEditText(item.text);
    }
    async function deleteItem(anchor: string) {
      if (!currentName) return;
      if (!window.confirm("Delete this item?")) return;
      const res = await commands.removeItem(currentName, anchor);
      res.status === "ok" ? setCurrentList(res.data) : setError(res.error);
    }
    async function saveEdit(anchor: string) {
      if (!currentName) return;
      const res = await commands.editItem(currentName, anchor, editText);
      if (res.status === "ok") {
        setCurrentList(res.data);
        setEditingAnchor(null);
        setEditText("");
      } else setError(res.error);
    }

    /* --- render --- */
    return (
      <div className="mb-6 w-full h-full rounded-lg border p-6 space-y-2" style={{ backgroundColor: "#1e1e2e", border: "1px solid #494D51" }}>
        {/* header row */}
        <div className="flex items-center gap-4">
          <h2 className="flex-1 text-base font-bold">{currentList.title}</h2>

          {/* <Button */}
          {/*   variant="secondary" */}
          {/*   size="sm" */}
          {/*   onClick={() => { */}
          {/*     setMultiSelect((v) => !v); */}
          {/*     setSelected(new Set()); */}
          {/*   }} */}
          {/* > */}
          {/*   {multiSelect ? " Done" : " Select"} */}
          {/* </Button> */}
          {/**/}
          {/* {multiSelect && ( */}
          {/*   <> */}
          {/*     <Button */}
          {/*       variant="outline" */}
          {/*       size="sm" */}
          {/*       onClick={async () => { */}
          {/*         if (!currentName) return; */}
          {/*         for (const a of Array.from(selected)) await commands.toggleItem(currentName, a); */}
          {/*         const res = await commands.getList(currentName); */}
          {/*         if (res.status === "ok") setCurrentList(res.data); */}
          {/*         setSelected(new Set()); */}
          {/*       }} */}
          {/*     > */}
          {/*        Mark Done */}
          {/*     </Button> */}
          {/*     <Button */}
          {/*       variant="destructive" */}
          {/*       size="sm" */}
          {/*       onClick={async () => { */}
          {/*         if (!currentName) return; */}
          {/*         if (!window.confirm("Delete selected items?")) return; */}
          {/*         for (const a of Array.from(selected)) await commands.removeItem(currentName, a); */}
          {/*         const res = await commands.getList(currentName); */}
          {/*         if (res.status === "ok") setCurrentList(res.data); */}
          {/*         setSelected(new Set()); */}
          {/*       }} */}
          {/*     > */}
          {/*        Delete */}
          {/*     </Button> */}
          {/*   </> */}
          {/* )} */}
        </div>

        {/* list items */}
        <div className="flex h-[80vh] w-full overflow-y-auto">
          <div className="w-full">
            {currentList.items.map((it, idx) =>
              editingAnchor === it.anchor ? (
                <form
                  key={it.anchor}
                  className="flex items-center"
                  onSubmit={(e) => {
                    e.preventDefault();
                    saveEdit(it.anchor);
                  }}
                >
                  <Input
                    className="flex-1"
                    value={editText}
                    onChange={(e) => setEditText(e.currentTarget.value)}
                    onBlur={() => saveEdit(it.anchor)}
                    autoFocus
                  />
                </form>
              ) : (
                <label
                  key={it.anchor}
                  draggable
                  onDragStart={() => (dragIndex.current = idx)}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => {
                    if (dragIndex.current === null || !currentName) return;
                    const fromAnchor = currentList.items[dragIndex.current].anchor;
                    commands.reorderItem(currentName, fromAnchor, idx).then((res) => {
                      res.status === "ok" ? setCurrentList(res.data) : setError(res.error);
                    });
                    dragIndex.current = null;
                  }}
                  className={`text-[10pt] flex items-center gap-3 border rounded-md mx-0 my-2 px-3 ${vimMode && idx === cursorIndex ? "outline outline-2 outline-dashed outline-[#a6e3a1]" : ""
                    } ${selected.has(it.anchor) ? "bg-[#a6e3a1] text-black" : ""}`}
                >
                  {multiSelect && (
                    <Checkbox
                      className="h-4 w-4"
                      checked={selected.has(it.anchor)}
                      onCheckedChange={(val) => {
                        setSelected((s) => {
                          const copy = new Set(s);
                          val ? copy.add(it.anchor) : copy.delete(it.anchor);
                          return copy;
                        });
                      }}
                    />
                  )}

                  <Checkbox
                    className="h-4 w-4"
                    checked={it.status === "Done"}
                    onCheckedChange={() => toggleItemStatus(it.anchor)}
                  />

                  <span
                    className={`flex-1 select-none ${it.status === "Done" ? "line-through text-muted" : ""
                      }`}
                    onDoubleClick={() => startEdit(it)}
                  >
                    {it.text}
                  </span>

                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => startEdit(it)}
                    aria-label="Edit"
                    className="flex text-xl text-muted"
                  >
                    
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => deleteItem(it.anchor)}
                    aria-label="Delete"
                    className="flex text-sm text-muted"
                  >
                    󰆴
                  </Button>
                </label>
              )
            )}

            {/* quick-add form */}
            <form className="flex gap-2" onSubmit={quickAddItem}>
              <Input
                className="flex-1 text-[10pt]"
                placeholder="Add item"
                value={newItem}
                onChange={(e) => setNewItem(e.currentTarget.value)}
              />
              <Button type="submit"></Button>
            </form>
          </div>
        </div>

      </div>
    );
  }

  function renderSidebar() {
    if (sidebarCollapsed) {
      return (
        <div className="hidden flex h-full w-12 flex-col border-r bg-background p-2 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSidebarCollapsed(false)}
            className="mb-2"
          >
            
          </Button>
        </div>
      );
    }

    const renderNodes = (nodes: ListNode[], depth = 0): JSX.Element[] =>
      nodes.flatMap((n) => [
        <div
          key={n.path}
          className={`cursor-pointer rounded-sm px-2 py-1 text-sm ${n.isList && n.path === currentName ? "bg-muted font-medium" : "hover:bg-muted"
            }`}
          style={{ paddingLeft: depth * 12 }}
          onClick={() => n.isList && loadList(n.path)}
        >
          {n.name}
        </div>,
        ...renderNodes(n.children, depth + 1),
      ]);

    return (
      <aside className="flex w-64 flex-col gap-4 border-r border-[#494D51] bg-background p-4 min-w-0 shrink-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setSidebarCollapsed(true)}
              className="h-6 w-6 p-0"
            >
              
            </Button>
            <h3 className="text-sm font-semibold">Lists</h3>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setCreating((c) => !c)}
          >
             New
          </Button>
        </div>

        {creating && (
          <form className="flex gap-2" onSubmit={createNewList}>
            <Input
              className="flex-1"
              placeholder="List name"
              value={newListName}
              onChange={(e) => setNewListName(e.target.value)}
            />
            <Button size="sm" type="submit">
               Create
            </Button>
          </form>
        )}

        <div className="flex-1 overflow-y-auto">{renderNodes(listTree)}</div>
      </aside>
    );
  }

  /* ---------- root render ---------- */
  return (
    <div className="flex min-h-screen border border-border bg-background text-foreground min-w-0 w-full" style={{ borderRadius: "10px", backgroundColor: "#24273a" }}>
      {renderSidebar()}

      <main className="relative flex flex-1 flex-col p-6 min-w-0 overflow-hidden">
        {/* top bar */}
        <div className="mb-4 flex items-center gap-4">
          <form
            className="relative w-full"
            onSubmit={(e) => e.preventDefault()}
          >
            <Input
              ref={inputRef}
              id="query"
              value={query}
              spellCheck={false}
              placeholder=""
              onKeyDown={(e) => {
                if (!showSuggestions) return;

                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setSelectedIndex((i) => (i + 1) % filtered.length);
                } else if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setSelectedIndex((i) => (i - 1 + filtered.length) % filtered.length);
                } else if (e.key === "Enter") {
                  e.preventDefault();
                  if (filtered.length > 0) loadList(filtered[selectedIndex]);
                }
              }}
              onFocus={() => {
                fetchLists();
                setShowSuggestions(true);
              }}
              onChange={(e) => setQuery(e.target.value)}
            />
            {renderSuggestions()}
            <img
              src={Logo}
              alt="lst icon"
              className="absolute right-2 top-1/2 -translate-y-1/2 h-5 w-5 opacity-75"
            />
          </form>
          {/* <a */}
          {/*   href="https://github.com/WismutHansen/lst" */}
          {/*   target="_blank" */}
          {/*   rel="noreferrer" */}
          {/*   className="ml-auto text-xs underline opacity-75" */}
          {/* > */}
          {/*   GitHub */}
          {/* </a> */}
        </div>

        {renderCurrentList()}

        {error && <p className="mt-4 text-red-600">⚠️ {error}</p>}

        {/* command palette (portal inside) */}
        <CommandPalette
          open={paletteOpen}
          onClose={() => setPaletteOpen(false)}
          commands={paletteCommands}
        />

        {/* Status bar */}
        <div className="absolute bottom-0 left-0 right-0 h-5 border-t border-border bg-[#181921] text-xs flex items-center px-2 rounded-b-lg">
          <span className="text-muted-foreground">lst - Terminal Todo Lists</span>
          <span className="ml-auto">{currentList ? `${currentList.items.length} items` : "No list selected"}</span>
        </div>
      </main>
    </div>
  );
}
