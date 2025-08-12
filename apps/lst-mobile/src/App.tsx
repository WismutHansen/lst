
import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import Logo from "./assets/logo.png";
import { commands, type List, type ListItem, type Category } from "./bindings";
import { CommandPalette, PaletteCommand } from "./components/CommandPalette";
import { MobileNotesPanel } from "./components/MobileNotesPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { SyncStatusIndicator } from "./components/SyncStatusIndicator";
import { MobileThemeSelector } from "./components/MobileThemeSelector";
import { Folder as FolderIcon, List as ListIcon, FileText, Clipboard, Settings, Menu, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";

import { useTheme } from "./hooks/useTheme";

interface ListNode {
  name: string;
  path: string;
  isList: boolean;
  children: ListNode[];
}

/* ---------- helpers ---------- */
function extractDate(name: string): Date | null {
  // Matches YYYY-MM-DD or YYYYMMDD
  const match = name.match(/(\d{4}-\d{2}-\d{2}|\d{8})/);
  if (match) {
    const dateStr = match[0].replace(/-/g, "");
    const year = parseInt(dateStr.substring(0, 4), 10);
    const month = parseInt(dateStr.substring(4, 6), 10) - 1; // Month is 0-indexed
    const day = parseInt(dateStr.substring(6, 8), 10);
    return new Date(year, month, day);
  }
  return null;
}

function buildListTree(paths: string[], sortOrder: "name" | "date-asc" | "date-desc"): ListNode[] {
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
      .sort((a: ListNode, b: ListNode) => {
        if (sortOrder === "name") {
          return a.name.localeCompare(b.name);
        }
        const dateA = extractDate(a.name);
        const dateB = extractDate(b.name);

        if (dateA && dateB) {
          return sortOrder === "date-asc"
            ? dateA.getTime() - dateB.getTime()
            : dateB.getTime() - dateA.getTime();
        }
        if (dateA) return -1; // lists with dates first
        if (dateB) return 1;
        return a.name.localeCompare(b.name);
      });
  return convert(root);
}
// ---------- date helpers ----------

// Format 2025-06-23 → "20250623"
const fmt = (date: Date): string =>
  date.toISOString().slice(0, 10).replace(/-/g, "");

// Keywords we support
type DateKeyword = "today" | "yesterday" | "tomorrow";

/**
 * Resolve "today" / "yesterday" / "tomorrow" to YYYYMMDD.
 * If the word isn’t one of those keywords it’s returned unchanged.
 */
export function resolveDateKeyword(word: string): string {
  const now = new Date(); // Europe/Berlin local time by default

  switch (word.toLowerCase() as DateKeyword) {
    case "today":
      return fmt(now);

    case "yesterday": {
      const d = new Date(now);
      d.setDate(d.getDate() - 1);
      return fmt(d);
    }

    case "tomorrow": {
      const d = new Date(now);
      d.setDate(d.getDate() + 1);
      return fmt(d);
    }

    default:
      return word; // leave untouched
  }
}

/**
 * Replace standalone keywords in a free-text query.
 *   "backup from yesterday" ➜ "backup from 20250622"
 */
export function translateQuery(q: string): string {
  return q.replace(/\b(today|yesterday|tomorrow)\b/gi, resolveDateKeyword);
}
/* ---------- component ---------- */
export default function App() {
  /* refs & state */
  const inputRef = useRef<HTMLInputElement>(null);
  const addItemRef = useRef<HTMLInputElement>(null);
  const listContainerRef = useRef<HTMLDivElement>(null);

  useTheme(); // Re-enabled with better error handling

  const [query, setQuery] = useState("");
  const [lists, setLists] = useState<string[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [currentList, setCurrentList] = useState<List | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentName, setCurrentName] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newListName, setNewListName] = useState("");
  const [isDisabled, setIsDisabled] = useState(false);
  const [newItem, setNewItem] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [editingAnchor, setEditingAnchor] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [selected] = useState<Set<string>>(new Set());
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [sortOrder, setSortOrder] = useState<"name" | "date-asc" | "date-desc">("name");
  const [currentView, setCurrentView] = useState<"lists" | "notes" | "settings">("lists");
  const [dropdownOpen, setDropdownOpen] = useState(false);



  const dragIndex = useRef<number | null>(null);

  /* ---------- folder management ---------- */
  function toggleFolder(path: string) {
    setExpandedFolders(prev => {
      const newSet = new Set(prev);
      if (newSet.has(path)) {
        newSet.delete(path);
      } else {
        newSet.add(path);
      }
      return newSet;
    });
  }

  /* ---------- helpers ---------- */
  // Get all items from both uncategorized and categorized sections
  function getAllItems(list: List | null): ListItem[] {
    if (!list) return [];
    return [
      ...(list.uncategorized_items ?? []),
      ...(list.categories ?? []).flatMap(c => c.items)
    ];
  }

  /* ---------- backend calls ---------- */
  async function reloadCurrentList() {
    if (!currentName) return;
    const res = await commands.getList(currentName);
    res.status === "ok" ? setCurrentList(res.data) : setError(res.error);
  }

  async function toggleItemStatus(anchor: string) {
    if (!currentName) return;
    const res = await commands.toggleItem(currentName, anchor);
    if (res.status === "ok") {
      setCurrentList(res.data);
    } else setError(res.error);
  }

  async function fetchLists() {
    console.log("📋 fetchLists called");
    const res = await commands.getLists();
    console.log("📋 getLists result:", res);
    if (res.status === "ok") {
      console.log("📋 Found", res.data.length, "lists:", res.data);
      setLists(res.data);
    } else {
      console.error("📋 Failed to get lists:", res.error);
      setError(res.error);
    }
  }

  const loadList = useCallback(async (name: string) => {
    console.log("📋 loadList called with name:", name);
    const res = await commands.getList(name);
    if (res.status === "ok") {
      console.log("✅ Successfully loaded list:", res.data.title);
      setCurrentList(res.data);
      setCurrentName(name);
      setShowSuggestions(false);
      setQuery("");
    } else {
      console.error("❌ Failed to load list:", res.error);
      setError(res.error);
    }
  }, []);

  /* ---------- mutations ---------- */
  async function createNewList(e: React.FormEvent) {
    e.preventDefault();
    console.log("📋 createNewList called with name:", newListName.trim());
    if (!newListName.trim()) return;
    
    console.log("📋 Calling commands.createList...");
    const res = await commands.createList(newListName.trim());
    console.log("📋 createList result:", res);
    
    if (res.status === "ok") {
      console.log("📋 List created successfully, refreshing lists...");
      await fetchLists();
      loadList(res.data.title);
      setNewListName("");
      setCreating(false);
    } else {
      console.error("📋 Failed to create list:", res.error);
      setError(res.error);
    }
  }

  async function quickAddItem(e: React.FormEvent) {
    e.preventDefault();
    console.log("📋 quickAddItem called - list:", currentName, "item:", newItem.trim());
    if (!currentName || !newItem.trim()) return;
    
    console.log("📋 Calling commands.addItem...");
    const res = await commands.addItem(currentName, newItem.trim(), null);
    console.log("📋 addItem result:", res);
    
    if (res.status === "ok") {
      console.log("📋 Item added successfully");
      setNewItem("");
      setCurrentList(res.data);
    } else {
      console.error("📋 Failed to add item:", res.error);
      setError(res.error);
    }
  }

  /* ---------- item-level helpers ---------- */
  function startEdit(item: ListItem) {
    setEditingAnchor(item.anchor);
    setEditText(item.text);
  }

  async function deleteItem(anchor: string) {
    if (!currentName) return;
    // if (!window.confirm("Delete this item?")) return;
    const res = await commands.removeItem(currentName, anchor);
    if (res.status === "ok") {
      setCurrentList(res.data);
    } else setError(res.error);
  }

  async function saveEdit(anchor: string) {
    if (!currentName) return;
    const res = await commands.editItem(currentName, anchor, editText);
    if (res.status === "ok") {
      setEditingAnchor(null);
      setEditText("");
      setCurrentList(res.data);
    } else setError(res.error);
  }



  /* ---------- derived ---------- */

  const resolvedQuery = useMemo(() => translateQuery(query), [query]);
  const filtered = useMemo(
    () =>
      resolvedQuery === "*"
        ? lists
        : lists.filter((l) =>
          l.toLowerCase().includes(resolvedQuery.toLowerCase())
        ),
    [resolvedQuery, lists]
  );
  const listTree = useMemo(() => buildListTree(lists, sortOrder), [lists, sortOrder]);

  const paletteCommands = useMemo<PaletteCommand[]>(
    () => [
      { label: "New List", action: () => {
        setCreating(true);
        setCurrentList(null); // Clear current list to show list browser
        setCurrentName(null);
      }},
      ...lists.map((l) => ({ label: `Open ${l}`, action: () => loadList(l) })),
    ],
    [lists]
  );

  /* ---------- lifecycle ---------- */
  // track screen size changes
  useEffect(() => {
    const listener = () =>
      window.addEventListener("resize", listener);
    return () => window.removeEventListener("resize", listener);
  }, []);



  useEffect(() => {
    fetchLists();
    openTodaysDailyList();
  }, []);

  // Debug creating state changes (remove this later)
  useEffect(() => {
    if (creating) console.log("🔍 Creating state changed to:", creating);
  }, [creating]);

  async function openTodaysDailyList() {
    const today = fmt(new Date());
    const dailyListName = `daily_lists/${today}_daily_list`;
    console.log("📅 openTodaysDailyList called for:", dailyListName);

    // Check if today's daily list exists in the current lists
    const res = await commands.getLists();
    if (res.status === "ok") {
      const exists = res.data.includes(dailyListName);
      console.log("📅 Daily list exists:", exists);

      if (exists) {
        // Open existing daily list
        console.log("📅 Opening existing daily list");
        loadList(dailyListName);
      } else {
        // Create new daily list using lst-cli command
        console.log("📅 Creating new daily list");
        try {
          const createRes = await commands.createList(dailyListName);
          if (createRes.status === "ok") {
            console.log("📅 Daily list created successfully");
            await fetchLists(); // Refresh the lists
            loadList(dailyListName);
          }
        } catch (error) {
          console.error("Failed to create daily list:", error);
        }
      }
    }
  }







  /* ---------- UI helpers ---------- */
  function renderSuggestions() {
    if (!showSuggestions || filtered.length === 0) return null;
    return (
      <div
        className="absolute left-0 top-[40px] z-20 w-full rounded-lg border overflow-y-auto"
        style={{ backgroundColor: "var(--muted)", border: "1px solid var(--border)" }}
      >
        {filtered.map((item, idx) => (
          <div
            key={item}
            className={`cursor-pointer px-3 py-2 text-xs ${idx === selectedIndex ? "bg-muted-foreground/20" : ""
              }`}
            onMouseDown={() => loadList(item)}
          >
            {item}
          </div>
        ))}
      </div>
    );
  }

  function renderMainContent() {
    if (currentView === "lists" && currentList) {
      return renderCurrentList();
    } else if (currentView === "lists" && !currentList) {
      return renderListBrowser();
    } else if (currentView === "notes") {
      return (
        <div className="h-full w-full rounded-lg border bg-muted/20 overflow-y-auto p-0.5">
          <MobileNotesPanel vimMode={false} theme="dark" />
        </div>
      );
    } else if (currentView === "settings") {
      return (
        <div className="h-full w-full rounded-lg border bg-muted/20 overflow-y-auto p-0.5">
          <SettingsPanel />
        </div>
      );
    }
    return null;
  }

  function renderListBrowser() {
    const renderNodes = (
      nodes: ListNode[],
      depth = 0
    ): JSX.Element[] =>
      nodes.flatMap((node) => {
        const isFolder = !node.isList;

        const common = "cursor-pointer rounded-sm py-2 px-3 text-sm flex items-center hover:bg-muted";
        const listClasses = node.isList && node.path === currentName ? "bg-muted font-medium" : "";

        return [
          <div
            key={node.path}
            className={`${common} ${listClasses}`}
            style={{ marginLeft: depth * 16 }}
            onClick={() =>
              node.isList ? loadList(node.path) : toggleFolder(node.path)
            }
          >
            {isFolder ? (
              <FolderIcon size={16} className="mr-2 flex-none" />
            ) : (
              <ListIcon size={16} className="mr-2 flex-none" />
            )}
            {node.name}
          </div>,
          ...(isFolder && expandedFolders.has(node.path) ? renderNodes(node.children, depth + 1) : []),
        ];
      });

    return (
      <div className="h-full w-full rounded-lg border p-4 bg-muted/20 overflow-y-auto">
        {creating && (
          <form className="flex gap-2 mb-4" onSubmit={createNewList}>
            <Input
              className="flex-1"
              placeholder="List name"
              value={newListName}
              onChange={(e) => setNewListName(e.target.value)}
              disabled={isDisabled}
              onClick={() => { setIsDisabled(false); }}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  (e.target as HTMLInputElement).blur();
                }
              }}
            />
            <Button size="sm" type="submit">
              Create
            </Button>
          </form>
        )}
        <div className="space-y-1">
          {renderNodes(listTree)}
        </div>
      </div>
    );
  }

  function renderCurrentList() {
    if (!currentList) return null;

    /* --- render --- */
    return (
      <div
        className="mb-6 w-full h-full rounded-lg border p-4 bg-muted/20"
        style={{ border: "1px solid var(--border)" }}
      >
        {/* header row */}
        {/* <div className="flex items-center gap-4"> */}
        {/*   <h2 className="flex-1 text-base font-bold">{currentList.title}</h2> */}
        {/* </div> */}

        {/* list items */}
        <div className="flex h-[calc(100vh-16rem)] sm:h-[calc(100vh-12rem)] w-full overflow-y-auto scroll-fade">
          <div ref={listContainerRef} className="w-full h-full">
            {/* Render uncategorized items first */}
            {(currentList.uncategorized_items ?? []).map((it, idx) =>
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
                    onKeyDown={(e) => {
                      if (e.key === "Escape") {
                        setEditingAnchor(null);
                        setEditText("");
                        e.preventDefault();
                        e.stopPropagation();
                      }
                    }}
                    autoFocus
                  />
                </form>
              ) : (
                <label
                  key={it.anchor}
                  data-item-index={idx}
                  draggable
                  onDragStart={() => (dragIndex.current = idx)}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => {
                    if (dragIndex.current === null || !currentName) return;
                    const allItems = getAllItems(currentList);
                    if (!allItems.length) return;
                    const fromAnchor = allItems[dragIndex.current].anchor;
                    commands
                      .reorderItem(currentName, fromAnchor, idx)
                      .then((res) => {
                        res.status === "ok"
                          ? setCurrentList(res.data)
                          : setError(res.error);
                      });
                    dragIndex.current = null;
                  }}
                  className={`text-[10pt]/4 flex items-center border-b min-h-10 py-2 mb-0 px-1 ${selected.has(it.anchor) ? "bg-primary text-primary-foreground" : ""}`}
                >
                  <Checkbox
                    className="h-4 w-4 hidden"
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

                  {/* <Button */}
                  {/*   variant="ghost" */}
                  {/*   size="icon" */}
                  {/*   onClick={() => startEdit(it)} */}
                  {/*   aria-label="Edit" */}
                  {/*   className="flex text-xl text-muted mr-2 h-6 w-6 rounded-sm" */}
                  {/* > */}
                  {/*    */}
                  {/* </Button> */}
                  {/* <Button */}
                  {/*   variant="ghost" */}
                  {/*   size="icon" */}
                  {/*   onClick={() => deleteItem(it.anchor)} */}
                  {/*   aria-label="Delete" */}
                  {/*   className="flex text-sm text-muted gap-0 h-6 w-6 rounded-sm" */}
                  {/* > */}
                  {/*   󰆴 */}
                  {/* </Button> */}
                </label>
              )
            )}

            {/* Render categorized items */}
            {(currentList.categories ?? []).map((category) => (
              <div key={category.name} className="mt-4">
                {/* Category header */}
                <div className="flex items-center gap-2 mb-2 pb-1 border-b border-border">
                  <h3 className="text-sm font-semibold text-primary">{category.name}</h3>
                  <span className="text-xs text-muted-foreground">({category.items.length})</span>
                </div>

                {/* Category items */}
                {category.items.map((it, idx) => {
                  const globalIdx = (currentList.uncategorized_items?.length ?? 0) +
                    (currentList.categories ?? [])
                      .slice(0, (currentList.categories ?? []).findIndex(c => c.name === category.name))
                      .reduce((acc, cat) => acc + cat.items.length, 0) + idx;

                  return editingAnchor === it.anchor ? (
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
                        onKeyDown={(e) => {
                          if (e.key === "Escape") {
                            setEditingAnchor(null);
                            setEditText("");
                            e.preventDefault();
                            e.stopPropagation();
                          }
                        }}
                        autoFocus
                      />
                    </form>
                  ) : (
                    <label
                      key={it.anchor}
                      data-item-index={globalIdx}
                      draggable
                      onDragStart={() => (dragIndex.current = globalIdx)}
                      onDragOver={(e) => e.preventDefault()}
                      onDrop={() => {
                        if (dragIndex.current === null || !currentName) return;
                        const allItems = getAllItems(currentList);
                        if (!allItems[dragIndex.current]) return;
                        const fromAnchor = allItems[dragIndex.current].anchor;
                        commands
                          .reorderItem(currentName, fromAnchor, globalIdx)
                          .then((res) => {
                            res.status === "ok"
                              ? setCurrentList(res.data)
                              : setError(res.error);
                          });
                        dragIndex.current = null;
                      }}
                      className={`text-[10pt]/4 flex items-center border-b min-h-10 py-2 mb-0 px-1 ${selected.has(it.anchor) ? "bg-primary text-primary-foreground" : ""}`}
                    >
                      <Checkbox
                        className="h-4 w-4 hidden"
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
                    </label>
                  );
                })}
              </div>
            ))}

            {/* quick-add form */}
            <form className="flex gap-2 border-b" onSubmit={quickAddItem}>
              <Input
                ref={addItemRef}
                className="flex-1 text-[10pt] border-none"
                placeholder="Add item"
                value={newItem}
                onChange={(e) => setNewItem(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") {
                    addItemRef.current?.blur();
                    e.preventDefault();
                    e.stopPropagation();
                  }
                }}
              />
              <Button type="submit" variant="ghost"></Button>
            </form>
          </div>
        </div>
      </div>
    );
  }



  /* ---------- root render ---------- */
  const items = getAllItems(currentList);
  return (
    <div
      className="flex h-screen bg-background text-foreground min-w-0 w-full overflow-hidden flex-col"
      style={{ backgroundColor: "var(--background)" }}
    >
      {/* Top bar - always visible */}
      <div className="flex-shrink-0 px-4 pb-2" style={{ paddingTop: "env(safe-area-inset-top, 44px)" }}>
        <div className="flex items-center gap-4">
          <form
            className="flex w-full items-center"
            onSubmit={(e) => e.preventDefault()}
          >
            <div className="flex relative items-center">
              <Button
                variant="outline"
                className="mr-2 h-10 w-10 p-0 flex-shrink-0"
                onClick={() => setDropdownOpen(!dropdownOpen)}
              >
                <Menu className="h-4 w-4 p-0" />
              </Button>

              {dropdownOpen && (
                <>
                  <div
                    className="fixed inset-0 z-10"
                    onClick={() => setDropdownOpen(false)}
                  />
                  <div className="absolute top-10 left-0 z-20 min-w-[160px] rounded-md border bg-popover p-1 text-popover-foreground shadow-md">
                    <div className="px-2 py-1.5 text-sm font-semibold">Navigation</div>
                    <div className="h-px bg-muted my-1" />
                    <button
                      className="relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground w-full text-left"
                      onClick={() => {
                        setCurrentView("lists");
                        setDropdownOpen(false);
                      }}
                    >
                      <Clipboard className="mr-2 h-4 w-4" />
                      Lists
                    </button>
                    <button
                      className="relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground w-full text-left"
                      onClick={() => {
                        setCurrentView("notes");
                        setDropdownOpen(false);
                      }}
                    >
                      <FileText className="mr-2 h-4 w-4" />
                      Notes
                    </button>
                    <button
                      className="relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground w-full text-left"
                      onClick={() => {
                        setCurrentView("settings");
                        setDropdownOpen(false);
                      }}
                    >
                      <Settings className="mr-2 h-4 w-4" />
                      Settings
                    </button>
                    <div className="h-px bg-muted my-1" />
                    <button
                      className="relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground w-full text-left"
                      onClick={() => {
                        setCreating(true);
                        setCurrentView("lists");
                        setCurrentList(null); // Clear current list to show list browser
                        setCurrentName(null);
                        setDropdownOpen(false);
                      }}
                    >
                      <Plus className="mr-2 h-4 w-4" />
                      New List
                    </button>
                  </div>
                </>
              )}
            </div>

            <Input
              ref={inputRef}
              id="query"
              value={query}
              spellCheck={false}
              placeholder=""
              className="h-10"
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setShowSuggestions(false);
                  setQuery("");
                  inputRef.current?.blur();
                  e.preventDefault();
                  return;
                }

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
                fetchLists(); // still refresh data on focus
              }}
              onChange={(e) => {
                const val = e.target.value;
                setQuery(val);

                const hasText = val.trim().length > 0;
                setShowSuggestions(hasText);

                if (!hasText) {
                  setSelectedIndex(0); // reset highlight when list closes
                }
              }}
            />
            {renderSuggestions()}

            <div
              className="border ml-2 flex items-center justify-center h-10 w-10 flex-shrink-0 rounded-md"
            >
              <img
                src={Logo}
                alt="lst icon"
                className="opacity-75 w-6 h-6 object-contain"
              />
            </div>
          </form>
        </div>
      </div>



      {/* Main content area - shows different content based on currentView */}
      <main className="flex-1 px-4 pb-4 min-w-0 overflow-hidden">
        {renderMainContent()}
      </main>

      {/* Status bar - always visible */}
      <div
        className="flex-shrink-0 border-t border-border bg-muted/20 text-xs flex items-center px-4 py-2 mobile-safe-area-bottom min-h-[44px]"
      >
        <span className="text-secondary-foreground truncate pr-4">
          lst {currentList ? `- ${currentList.title}.md` : ""}
        </span>
        <span className="text-muted-foreground">
          {error && <p className="ml-2 text-destructive">{error}</p>}
        </span>
        <div className="ml-auto flex items-center gap-2">
          <SyncStatusIndicator />
          <span className="text-nowrap">
            {currentList ? `${items.length} items` : "No list selected"}
          </span>
        </div>
      </div>



      {/* Command palette */}
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        commands={paletteCommands}
      />
    </div>
  );
}
