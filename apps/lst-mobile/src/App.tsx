import { listen } from "@tauri-apps/api/event";
import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import Logo from "./assets/logo.png";
import { commands, type List, type ListItem, type Category } from "./bindings";
import { CommandPalette, PaletteCommand } from "./components/CommandPalette";
import { MobileNotesPanel } from "./components/MobileNotesPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { SyncStatusIndicator } from "./components/SyncStatusIndicator";
import { MobileThemeSelector } from "./components/MobileThemeSelector";
import { Folder as FolderIcon, List as ListIcon, FileText, Clipboard, Settings, Menu } from "lucide-react";
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

  useTheme();

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

  /* ---------- sidebar & responsive ---------- */
  // sidebar is collapsed by default
  const [sidebarCollapsed, setSidebarCollapsed] = useState(true);
  // track if screen is mobile-sized (≤640 px)
  // keyboard focus index inside sidebar
  const [sidebarCursor, setSidebarCursor] = useState(0);

  /* ----- vim-like mode (unchanged) ----- */
  const [vimMode, setVimMode] = useState(false);
  const [leaderKey, setLeaderKey] = useState(" ");
  const [mode, setMode] = useState<"normal" | "edit">("edit");
  const [cursorIndex, setCursorIndex] = useState(0);
  const [leaderActive, setLeaderActive] = useState(false);
  const [leaderSeq, setLeaderSeq] = useState("");
  const [gPressed, setGPressed] = useState(false);
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
    const res = await commands.getLists();
    res.status === "ok" ? setLists(res.data) : setError(res.error);
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
    const res = await commands.addItem(currentName, newItem.trim(), null);
    if (res.status === "ok") {
      setNewItem("");
      setCurrentList(res.data);
    } else setError(res.error);
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

  /* ---------- scroll helpers ---------- */
  function scrollToItem(index: number) {
    if (!listContainerRef.current || !currentList) return;

    const allItems = getAllItems(currentList);
    // If navigating to add item (index === allItems.length)
    if (allItems.length === 0) return;
    if (index === allItems.length) {
      // Scroll the container to the bottom to show the add item form
      const container = listContainerRef.current.parentElement; // The scrollable div
      if (container) {
        container.scrollTo({
          top: container.scrollHeight,
          behavior: "smooth"
        });
      }
      return;
    }

    // For regular list items, find the element by index
    const listItems = listContainerRef.current.querySelectorAll("[data-item-index]");
    const targetItem = listItems[index] as HTMLElement;

    if (targetItem) {
      targetItem.scrollIntoView({
        behavior: "smooth",
        block: "nearest"
      });
    }
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
  // flattened sidebar list for keyboard nav
  const flatSidebarItems: { path: string; isList: boolean }[] = useMemo(() => {
    const dfs = (nodes: ListNode[]): { path: string; isList: boolean }[] =>
      nodes.flatMap((n) => {
        const isFolder = !n.isList;
        const children = (isFolder && expandedFolders.has(n.path)) ? dfs(n.children) : [];
        return [
          { path: n.path, isList: n.isList },
          ...children,
        ];
      });
    return dfs(listTree);
  }, [listTree, expandedFolders]);

  const paletteCommands = useMemo<PaletteCommand[]>(
    () => [
      { label: "New List", action: () => setCreating(true) },
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
    (async () => {
      const res = await commands.getUiConfig();
      if (res.status === "ok") { const { vim_mode = false, leader_key = "<leader>" } = res.data; setVimMode(vim_mode); setLeaderKey(leader_key); setMode(vim_mode ? "normal" : "edit"); }
    })();
  }, []);

  useEffect(() => {
    fetchLists();
    openTodaysDailyList();
  }, []);

  async function openTodaysDailyList() {
    const today = fmt(new Date());
    const dailyListName = `daily_lists/${today}_daily_list`;

    // Check if today's daily list exists in the current lists
    const res = await commands.getLists();
    if (res.status === "ok") {
      const exists = res.data.includes(dailyListName);

      if (exists) {
        // Open existing daily list
        loadList(dailyListName);
      } else {
        // Create new daily list using lst-cli command
        try {
          const createRes = await commands.createList(dailyListName);
          if (createRes.status === "ok") {
            await fetchLists(); // Refresh the lists
            loadList(dailyListName);
          }
        } catch (error) {
          console.error("Failed to create daily list:", error);
        }
      }
    }
  }

  useEffect(() => {
    console.log("🎧 Setting up event listener for 'switch-list'");
    const unlisten = listen<string>("switch-list", (event) => {
      console.log("📨 Received 'switch-list' event with payload:", event.payload);
      loadList(event.payload);
    });
    return () => {
      console.log("🔇 Cleaning up 'switch-list' event listener");
      unlisten.then((fn) => fn());
    };
  }, [loadList]);

  // Test event listener
  useEffect(() => {
    console.log("🧪 Setting up test event listener");
    const unlisten = listen<string>("test-event", (event) => {
      console.log("🎉 Received test event with payload:", event.payload);
      alert("Test event received: " + event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Auto-refresh mechanism
  useEffect(() => {
    const refreshInterval = setInterval(async () => {
      // Refresh the lists
      await fetchLists();

      // If we have a current list loaded, refresh it too
      if (currentName) {
        const res = await commands.getList(currentName);
        if (res.status === "ok") {
          setCurrentList(res.data);
        }
      }
    }, 2000); // Refresh every 2 seconds

    return () => clearInterval(refreshInterval);
  }, [currentName]);

  /* ---------- keybindings ---------- */
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      // Check if any input is focused - if so, don't process vim commands
      const activeElement = document.activeElement;
      const isInputFocused = activeElement && (
        activeElement.tagName === "INPUT" ||
        activeElement.tagName === "TEXTAREA" //||
        // activeElement.contentEditable === "true"
      );

      // toggle sidebar with Ctrl-b
      if (e.key.toLowerCase() === "/") {

        e.preventDefault();
        return;
      }

      // toggle sidebar with Ctrl-b
      if (e.ctrlKey && e.key.toLowerCase() === "b") {
        setSidebarCollapsed((c) => !c);
        e.preventDefault();
        return;
      }

      // sidebar navigation (when open)
      if (!sidebarCollapsed) {
        const next = (delta: number) => {
          setSidebarCursor((i) =>
            (i + delta + flatSidebarItems.length) % flatSidebarItems.length
          );
        };

        // Vim or arrow keys
        if (vimMode && mode === "normal") {
          if (["j", "k"].includes(e.key)) {
            next(e.key === "j" ? 1 : -1);
            e.preventDefault();
            return;
          }
          if (e.key === "l") {
            const item = flatSidebarItems[sidebarCursor];
            if (item?.isList) loadList(item.path);
            e.preventDefault();
            return;
          }
          if (e.key === " ") {
            const item = flatSidebarItems[sidebarCursor];
            if (item && !item.isList) toggleFolder(item.path);
            e.preventDefault();
            return;
          }
        } else {
          if (["ArrowDown", "ArrowUp"].includes(e.key)) {
            next(e.key === "ArrowDown" ? 1 : -1);
            e.preventDefault();
            return;
          }
          if (e.key === "ArrowRight") {
            const item = flatSidebarItems[sidebarCursor];
            if (item?.isList) loadList(item.path);
            e.preventDefault();
            return;
          }
          if (e.key === " ") {
            const item = flatSidebarItems[sidebarCursor];
            if (item && !item.isList) toggleFolder(item.path);
            e.preventDefault();
            return;
          }
        }
      }

      // List item navigation in vim mode (only if no input is focused)
      if (vimMode && currentList && sidebarCollapsed && !isInputFocused) {
        // ESC key - exit edit mode to normal mode
        if (e.key === "Escape") {
          if (mode === "edit") {
            setMode("normal");
            setEditingAnchor(null);
            setEditText("");
            e.preventDefault();
            return;
          }
        }

        // Normal mode keybindings
        if (mode === "normal") {
          // j/k navigation within list items (including add item input)
          const allItems = getAllItems(currentList);
          const maxIndex = allItems.length; // Add item is at allItems.length
          if (e.key === "j") {
            const newIndex = Math.min(cursorIndex + 1, maxIndex);
            setCursorIndex(newIndex);
            scrollToItem(newIndex);
            if (newIndex === maxIndex) {
              // Focus on add item input
              addItemRef.current?.focus();
            }
            e.preventDefault();
            return;
          }
          if (e.key === "k") {
            const newIndex = Math.max(cursorIndex - 1, 0);
            setCursorIndex(newIndex);
            scrollToItem(newIndex);
            if (cursorIndex === maxIndex) {
              // Moving up from add item, blur it
              addItemRef.current?.blur();
            }
            e.preventDefault();
            return;
          }

          // 'g' handling for 'gg' sequence
          if (e.key === "g") {
            if (gPressed) {
              // Second 'g' - jump to top
              setCursorIndex(0);
              scrollToItem(0);
              addItemRef.current?.blur();
              setGPressed(false);
            } else {
              // First 'g' - wait for second
              setGPressed(true);
              // Clear the g-pressed state after a timeout
              setTimeout(() => setGPressed(false), 1000);
            }
            e.preventDefault();
            return;
          }

          // 'G' to jump to bottom (Add item)
          if (e.key === "G") {
            setCursorIndex(maxIndex);
            scrollToItem(maxIndex);
            addItemRef.current?.focus();
            setGPressed(false); // Clear any pending g press
            e.preventDefault();
            return;
          }

          // Reset g-pressed state on any other key
          if (gPressed && e.key !== "g") {
            setGPressed(false);
          }

          // 'i' to enter edit mode on current item
          if (e.key === "i") {
            const allItems = getAllItems(currentList);
            const currentItem = allItems[cursorIndex];
            if (currentItem) {
              startEdit(currentItem);
              setMode("edit");
            }
            e.preventDefault();
            return;
          }

          // Leader key combinations
          if (leaderActive) {
            if (leaderSeq === "" && e.key === "d") {
              setLeaderSeq("d");
              e.preventDefault();
              return;
            }

            // 'dd' to delete current item
            if (leaderSeq === "d" && e.key === "d") {
              const allItems = getAllItems(currentList);
              const currentItem = allItems[cursorIndex];
              if (currentItem) {
                deleteItem(currentItem.anchor);
              }
              setLeaderActive(false);
              setLeaderSeq("");
              e.preventDefault();
              return;
            }

            // 'md' to mark as done
            if (leaderSeq === "m" && e.key === "d") {
              const allItems = getAllItems(currentList);
              const currentItem = allItems[cursorIndex];
              if (currentItem) {
                toggleItemStatus(currentItem.anchor);
              }
              setLeaderActive(false);
              setLeaderSeq("");
              e.preventDefault();
              return;
            }

            if (e.key === "m") {
              setLeaderSeq("m");
              e.preventDefault();
              return;
            }

            // Reset on any other key
            setLeaderActive(false);
            setLeaderSeq("");
          } else if (e.key === leaderKey) {
            // Activate leader key
            setLeaderActive(true);
            setLeaderSeq("");
            e.preventDefault();
            return;
          }
        }
      }

      /* other key handling (existing logic, omitted) */
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    sidebarCollapsed,
    flatSidebarItems,
    sidebarCursor,
    vimMode,
    mode,
    leaderActive,
    leaderSeq,
    leaderKey,
    currentList,
    cursorIndex,
    editingAnchor,
    currentName,
    gPressed,
  ]);

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

  function renderCurrentList() {
    if (!currentList) return null;

    /* --- render --- */
    return (
      <div
        className="mb-6 w-full h-full rounded-lg border p-4"
        style={{ backgroundColor: "var(--card)", border: "1px solid var(--border)" }}
      >
        {/* header row */}
        {/* <div className="flex items-center gap-4"> */}
        {/*   <h2 className="flex-1 text-base font-bold">{currentList.title}</h2> */}
        {/* </div> */}

        {/* list items */}
        <div className="flex h-[80vh] w-full overflow-y-auto scroll-fade">
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
                      if (e.key === "Escape" && vimMode) {
                        setMode("normal");
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
                  className={`text-[10pt]/4 flex items-center border-b min-h-10 py-2 mb-0 px-1 ${vimMode && mode === "normal" && idx === cursorIndex
                    ? "border-b border-primary"
                    : ""
                    } ${selected.has(it.anchor) ? "bg-primary text-primary-foreground" : ""}`}
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
                          if (e.key === "Escape" && vimMode) {
                            setMode("normal");
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
                      className={`text-[10pt]/4 flex items-center border-b min-h-10 py-2 mb-0 px-1 ${vimMode && mode === "normal" && globalIdx === cursorIndex
                        ? "border-b border-primary"
                        : ""
                        } ${selected.has(it.anchor) ? "bg-primary text-primary-foreground" : ""}`}
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
            <form className={`flex gap-2 border-b ${vimMode && mode === "normal" && cursorIndex === getAllItems(currentList).length
              ? "border-b border-primary"
              : ""
              }`} onSubmit={quickAddItem}>
              <Input
                ref={addItemRef}
                className="flex-1 text-[10pt] border-none"
                placeholder="Add item"
                value={newItem}
                onChange={(e) => setNewItem(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape" && vimMode) {
                    setMode("normal");
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

  function renderSidebar() {
    if (sidebarCollapsed) {
      return (
        <aside className="hidden sm:flex w-12 flex-col gap-4 rounded-l-lg border-r border-border bg-background p-4 min-w-0 shrink-0">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setSidebarCollapsed(false)}
                className="h-6 w-6 p-0 pt-8"
              >󰞘
              </Button>
              <div className="h-8"></div>
            </div>
          </div>
        </aside>
      );
    }

    const renderNodes = (
      nodes: ListNode[],
      depth = 0
    ): JSX.Element[] =>
      nodes.flatMap((node) => {
        const isFolder = !node.isList;
        const flatIndex = flatSidebarItems.findIndex(
          (f) => f.path === node.path && f.isList === node.isList
        );
        const highlighted = flatIndex === sidebarCursor;

        // ── class helpers ──────────────────────────────
        const common = "cursor-pointer rounded-sm py-1 pl-2 text-sm flex items-center";
        const listClasses =
          node.isList && node.path === currentName
            ? "bg-muted font-medium"
            : highlighted
              ? "bg-muted-foreground/20"
              : "hover:bg-muted";

        const folderClasses =
          highlighted ? "bg-muted-foreground/10" : "hover:bg-muted/50";

        return [
          <div
            key={node.path}
            className={`${common} ${isFolder ? folderClasses : listClasses}`}
            style={{ marginLeft: depth * 12 }}
            onClick={() =>
              node.isList ? loadList(node.path) : toggleFolder(node.path)
            }
          >
            {isFolder ? (
              <FolderIcon size={16} className="mr-1 flex-none" />
            ) : (
              <ListIcon size={16} className="mr-1 flex-none" />
            )}
            {node.name}
          </div>,
          ...(isFolder && expandedFolders.has(node.path) ? renderNodes(node.children, depth + 1) : []),
        ];
      });

    const sidebarContent = (
      <aside className="flex w-64 pl-2 flex-col gap-4 rounded-l-lg border-r border-border bg-background p-4 min-w-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setSidebarCollapsed(true)}
              className="h-7 w-7"
            >
              󰞗
            </Button>
            <div className="flex gap-1">
              <Button
                variant={currentView === "lists" ? "default" : "ghost"}
                size="sm"
                onClick={() => setCurrentView("lists")}
                className="h-7 px-2 text-xs"
              >
                <Clipboard className="h-3 w-3" />
              </Button>
              <Button
                variant={currentView === "notes" ? "default" : "ghost"}
                size="sm"
                onClick={() => setCurrentView("notes")}
                className="h-7 px-2 text-xs"
              >
                <FileText className="h-3 w-3" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setCreating((c) => !c)}
                className="h-7 w-7 px-2 text-xs"
              >
                
              </Button>
            </div>
          </div>
          <Button
            variant={currentView === "settings" ? "default" : "ghost"}
            size="sm"
            onClick={() => setCurrentView("settings")}
            className="h-7 px-2 text-xs"
          >
            <Settings className="h-3 w-3 mr-1" />
          </Button>
        </div>

        {currentView === "lists" && creating && (
          <form className="flex gap-2" onSubmit={createNewList}>
            <Input
              className="flex-1"
              placeholder="List name"
              value={newListName}
              onChange={(e) => setNewListName(e.target.value)}
              disabled={isDisabled} // 🔒 fully blocks input
              onClick={() => { setMode("edit"); setIsDisabled(false); }}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  if (vimMode) {
                    setMode("normal");
                    (e.target as HTMLInputElement).blur();
                  }
                }
              }}
            />
            < Button size="sm" type="submit" >
               Create
            </Button>
          </form>
        )
        }

        {currentView === "lists" && (
          <div className="flex-1 overflow-y-auto pl-2 w-auto">{renderNodes(listTree)}</div>
        )}
      </aside >
    );


    // desktop regular sidebar
    return sidebarContent;
  }

  const items = getAllItems(currentList);
  /* ---------- root render ---------- */
  return (
    <div
      className="flex min-h-screen border border-border bg-background text-foreground min-w-0 w-full"
      style={{ borderRadius: "0px", backgroundColor: "var(--background)" }}
    >
      {renderSidebar()}

      <main className="relative flex flex-1 flex-col p-6 min-w-0 overflow-hidden">
        {/* top bar */}
        <div className="mt-2 mb-2 flex items-center gap-4">
          <form
            className="flex w-full"
            onSubmit={(e) => e.preventDefault()}
          >
            <div>            <Button
              variant="outline"
              size="icon"
              onClick={() => setSidebarCollapsed(false)}
              className="sm:hidden mr-2"
            >
              <Menu></Menu>
            </Button>
            </div>

            <Input
              ref={inputRef}
              id="query"
              value={query}
              spellCheck={false}
              placeholder=""
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setShowSuggestions(false);
                  setQuery("");
                  inputRef.current?.blur();
                  if (vimMode) {
                    setMode("normal");
                  }
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
              className="border ml-2 flex items-center h-9 w-12"
            >
              <img
                src={Logo}
                alt="lst icon"
                className="opacity-75"
              />
            </div>
          </form>
        </div>

        {currentView === "lists" ? (
          renderCurrentList()
        ) : currentView === "notes" ? (
          <MobileNotesPanel vimMode={vimMode} theme="dark" />
        ) : (
          <SettingsPanel />
        )}


        {/* command palette (portal inside) */}
        <CommandPalette
          open={paletteOpen}
          onClose={() => setPaletteOpen(false)}
          commands={paletteCommands}
        />

      </main>
      {/* Status bar */}
      <div
        className="fixed bottom-0 left-0 right-0 h-5 border border-border bg-card text-xs flex items-center px-2 rounded-b-lg"
      >
        <span className="text-muted-foreground truncate pr-4">
          lst {currentList ? `- ${currentList.title}.md` : ""}
        </span>
        <span className="text-muted-foreground">
          {error && <p className="ml-2 text-red-600">{error}</p>}
        </span>
        <div className="ml-auto flex items-center gap-2">
          <SyncStatusIndicator />
          <span className="text-nowrap">
            {currentList ? `${items.length} items` : "No list selected"}
          </span>
        </div>
      </div>
    </div>
  );
}
