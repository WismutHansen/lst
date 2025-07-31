import { listen } from "@tauri-apps/api/event";
import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import Logo from "./assets/logo.png";
import { commands, type List, type ListItem, type Category } from "./bindings";
import { CommandPalette, PaletteCommand } from "./components/CommandPalette";
import { NotesPanel } from "./components/NotesPanel";
import { ThemeSelector } from "./components/ThemeSelector";
import { Folder as FolderIcon, List as ListIcon, FileText, Clipboard, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
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

function buildNotesTree(paths: string[], sortOrder: "name" | "date-asc" | "date-desc"): ListNode[] {
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
        if (dateA) return -1; // notes with dates first
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
  const listsContainerRef = useRef<HTMLDivElement>(null);
  const notesContainerRef = useRef<HTMLDivElement>(null);

  useTheme();

  const [query, setQuery] = useState("");
  const [lists, setLists] = useState<string[]>([]);
  const [notes, setNotes] = useState<string[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [currentList, setCurrentList] = useState<List | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [messageTimeoutId, setMessageTimeoutId] = useState<number | null>(null);
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
  const [currentView, setCurrentView] = useState<"lists" | "notes">("lists");
  const [vimStatus, setVimStatus] = useState<{ mode: string; status?: string } | null>(null);

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

  const loadLists = useCallback(async () => {
    await fetchLists();
  }, []);

  async function fetchNotes() {
    const res = await commands.getNotes();
    res.status === "ok" ? setNotes(res.data) : setError(res.error);
  }

  const showMessage = useCallback((text: string, duration: number = 3000) => {
    // Clear any existing timeout
    if (messageTimeoutId) {
      clearTimeout(messageTimeoutId);
    }

    // Set the new message
    setMessage(text);

    // Set timeout to clear the message
    const timeoutId = setTimeout(() => {
      setMessage(null);
      setMessageTimeoutId(null);
    }, duration);

    setMessageTimeoutId(timeoutId);
  }, [messageTimeoutId]);

  const loadList = useCallback(async (name: string) => {
    console.log("📋 loadList called with name:", name);
    const res = await commands.getList(name);
    if (res.status === "ok") {
      console.log("✅ Successfully loaded list:", res.data.title);
      setCurrentList(res.data);
      setCurrentName(name);
      setCurrentView("lists");
      setShowSuggestions(false);
      setQuery("");
      setVimStatus(null); // Clear vim status when switching to lists
    } else {
      console.error("❌ Failed to load list:", res.error);
      setError(res.error);
    }
  }, []);

  const loadNote = useCallback(async (name: string) => {
    console.log("📝 loadNote called with name:", name);
    setCurrentList(null);
    setCurrentName(name);
    setCurrentView("notes");
    setShowSuggestions(false);
    setQuery("");
    setVimStatus(null); // Clear vim status when switching notes
  }, []);

  /* ---------- mutations ---------- */
  async function createNewList(e: React.FormEvent) {
    e.preventDefault();
    if (!newListName.trim()) return;

    if (currentView === "lists") {
      const res = await commands.createList(newListName.trim());
      if (res.status === "ok") {
        await fetchLists();
        loadList(res.data.title);
        setNewListName("");
        setCreating(false);
      } else setError(res.error);
    } else {
      const res = await commands.createNoteCmd(newListName.trim());
      if (res.status === "ok") {
        await fetchNotes();
        loadNote(res.data.title);
        setNewListName("");
        setCreating(false);
      } else setError(res.error);
    }
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

  function scrollToSidebarItem(index: number) {
    // Get the current active container based on the view
    const activeContainer = currentView === "lists" ? listsContainerRef.current : notesContainerRef.current;
    
    if (!activeContainer) return;

    // Find the sidebar item by index using data-sidebar-index attribute
    const targetItem = activeContainer.querySelector(`[data-sidebar-index="${index}"]`) as HTMLElement;

    if (targetItem) {
      // Always scroll to ensure the highlighted item is visible
      // Using scrollIntoView with block: "center" for better visibility
      targetItem.scrollIntoView({
        behavior: "smooth",
        block: "center",
        inline: "nearest"
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
  const notesTree = useMemo(() => buildNotesTree(notes, sortOrder), [notes, sortOrder]);
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
    return currentView === "lists" ? dfs(listTree) : dfs(notesTree);
  }, [listTree, notesTree, expandedFolders, currentView]);

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
    fetchNotes();
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

  useEffect(() => {
    console.log("🎧 Setting up event listener for 'show-message'");
    const unlisten = listen<string>("show-message", (event) => {
      console.log("📨 Received 'show-message' event with payload:", event.payload);
      showMessage(event.payload);
    });
    return () => {
      console.log("🔇 Cleaning up 'show-message' event listener");
      unlisten.then((fn) => fn());
    };
  }, [showMessage]);

  // List updated event listener
  useEffect(() => {
    console.log("🎧 Setting up event listener for 'list-updated'");
    const unlisten = listen<string>("list-updated", (event) => {
      console.log("📨 Received 'list-updated' event with payload:", event.payload);
      // If the updated list is the currently loaded list, reload it
      if (currentName === event.payload) {
        loadList(event.payload);
      }
      // Also refresh the lists sidebar to show any new lists
      loadLists();
    });
    return () => {
      console.log("🔇 Cleaning up 'list-updated' event listener");
      unlisten.then((fn) => fn());
    };
  }, [currentName, loadList, loadLists]);

  // Note updated event listener
  useEffect(() => {
    console.log("🎧 Setting up event listener for 'note-updated'");
    const unlisten = listen<string>("note-updated", (event) => {
      console.log("📨 Received 'note-updated' event with payload:", event.payload);
      // Refresh the notes panel to show any changes
      // This will be handled by the NotesPanel component if it's listening
      // For now, we could show a message or trigger a refresh
      showMessage(`Note '${event.payload}' was updated`);
    });
    return () => {
      console.log("🔇 Cleaning up 'note-updated' event listener");
      unlisten.then((fn) => fn());
    };
  }, [showMessage]);

  // File changed event listener
  useEffect(() => {
    console.log("🎧 Setting up event listener for 'file-changed'");
    const unlisten = listen<string>("file-changed", (event) => {
      console.log("📨 Received 'file-changed' event with payload:", event.payload);
      // Generic file change handler - could be used for future enhancements
      // For now, just log it
      console.log("📁 File changed:", event.payload);
    });
    return () => {
      console.log("🔇 Cleaning up 'file-changed' event listener");
      unlisten.then((fn) => fn());
    };
  }, []);

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

  // Cleanup message timeout on unmount
  useEffect(() => {
    return () => {
      if (messageTimeoutId) {
        clearTimeout(messageTimeoutId);
      }
    };
  }, [messageTimeoutId]);

  // Auto-refresh mechanism
  useEffect(() => {
    const refreshInterval = setInterval(async () => {
      // Refresh the lists and notes
      await fetchLists();
      await fetchNotes();

      // If we have a current list loaded, refresh it too
      if (currentName && currentView === "lists") {
        const res = await commands.getList(currentName);
        if (res.status === "ok") {
          setCurrentList(res.data);
        }
      }
    }, 2000); // Refresh every 2 seconds

    return () => clearInterval(refreshInterval);
  }, [currentName, currentView]);

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

      // focus search bar with "/"
      if (e.key.toLowerCase() === "/" && !isInputFocused) {
        inputRef.current?.focus();
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
          setSidebarCursor((i) => {
            const newIndex = (i + delta + flatSidebarItems.length) % flatSidebarItems.length;
            // Use setTimeout to ensure the DOM has updated before scrolling
            setTimeout(() => scrollToSidebarItem(newIndex), 10);
            return newIndex;
          });
        };

        const selectCurrentItem = () => {
          const item = flatSidebarItems[sidebarCursor];
          if (item?.isList) {
            if (currentView === "lists") {
              loadList(item.path);
            } else {
              loadNote(item.path);
            }
          }
        };

        const toggleCurrentFolder = () => {
          const item = flatSidebarItems[sidebarCursor];
          if (item && !item.isList) toggleFolder(item.path);
        };

        // Vim keys
        if (vimMode && mode === "normal") {
          if (["j", "k"].includes(e.key)) {
            next(e.key === "j" ? 1 : -1);
            e.preventDefault();
            return;
          }
          if (e.key === "l") {
            selectCurrentItem();
            e.preventDefault();
            return;
          }
          if (e.key === " ") {
            toggleCurrentFolder();
            e.preventDefault();
            return;
          }
        }
        
        // Arrow keys (work in both vim and non-vim mode for sidebar)
        if (["ArrowDown", "ArrowUp"].includes(e.key)) {
          next(e.key === "ArrowDown" ? 1 : -1);
          e.preventDefault();
          return;
        }
        if (e.key === "ArrowRight") {
          selectCurrentItem();
          e.preventDefault();
          return;
        }
        if (e.key === " ") {
          toggleCurrentFolder();
          e.preventDefault();
          return;
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
                        if (!allItems[dragIndex.current]) return;
                        const fromAnchor = allItems[dragIndex.current].anchor;                    commands
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
                        const allItems = [
                          ...(currentList.uncategorized_items ?? []),
                          ...(currentList.categories ?? []).flatMap(c => c.items)
                        ];
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
        <aside className="hidden sm:flex w-12 flex-col gap-4 rounded-l-lg border-r border-border bg-background pl-2 pt-2 min-w-0 shrink-0">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setSidebarCollapsed(false)}
                className="mt-4 h-9 w-9 p-0"
              >
                󰞘
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
            data-sidebar-index={flatIndex}
            className={`${common} ${isFolder ? folderClasses : listClasses}`}
            style={{ marginLeft: depth * 12 }}
            onClick={() => {
              // Update the sidebar cursor to match the clicked item
              setSidebarCursor(flatIndex);
              
              if (node.isList) {
                currentView === "lists" ? loadList(node.path) : loadNote(node.path);
              } else {
                toggleFolder(node.path);
              }
            }}
          >
            {isFolder ? (
              <FolderIcon size={16} className="mr-1 flex-none" />
            ) : currentView === "lists" ? (
              <ListIcon size={16} className="mr-1 flex-none" />
            ) : (
              <FileText size={16} className="mr-1 flex-none" />
            )}
            {node.name}
          </div>,
          ...(isFolder && expandedFolders.has(node.path) ? renderNodes(node.children, depth + 1) : []),
        ];
      });

    const sidebarContent = (
      <aside className="flex w-64 pl-2 flex-col gap-4 rounded-l-lg border-r border-border bg-background mt-2 p-4 min-w-0 h-screen overflow-hidden">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setSidebarCollapsed(true)}
              className="h-9 w-9 p-0"
            >
              󰞗
            </Button>
          </div>
          <Input className="mx-2">
          </Input>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setCreating((c) => !c)}
            className="h-9 w-9"
          >
            <Plus />
          </Button>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">Theme:</span>
          <ThemeSelector />
        </div>

        {creating && (
            <form className="flex gap-2 mt-2" onSubmit={createNewList}>
              <Input
                className="flex-1"
                placeholder={currentView === "lists" ? "List name" : "Note name"}
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
              <Button size="sm" type="submit">
                Create
              </Button>
            </form>
          )}

          <Tabs defaultValue="lists" className="flex-1 flex flex-col">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="lists">Lists</TabsTrigger>
              <TabsTrigger value="notes">Notes</TabsTrigger>
            </TabsList>

            <TabsContent 
            value="lists" 
            ref={listsContainerRef}
            className="flex-1 overflow-y-auto pl-2 w-auto mt-2 min-h-0"
            onWheel={(e) => e.stopPropagation()}
            onScroll={(e) => e.stopPropagation()}
          >
            {renderNodes(listTree)}
          </TabsContent>

          <TabsContent 
            value="notes" 
            ref={notesContainerRef}
            className="flex-1 overflow-y-auto pl-2 w-auto mt-2 min-h-0"
            onWheel={(e) => e.stopPropagation()}
            onScroll={(e) => e.stopPropagation()}
          >
            {renderNodes(notesTree)}
          </TabsContent>
        </Tabs>
      </aside>
    );


    // desktop regular sidebar
    return sidebarContent;
  }

  const items = getAllItems(currentList);
  /* ---------- root render ---------- */
  return (
    <div
      className="flex min-h-screen border border-border bg-background text-foreground min-w-0 w-full"
      style={{ borderRadius: "10px", backgroundColor: "var(--background)" }}
    >
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
            <img
              src={Logo}
              alt="lst icon"
              className="absolute right-2 top-1/2 -translate-y-1/2 h-7 w-7 opacity-75"
            />
          </form>
        </div>

        {currentView === "lists" ? (
          renderCurrentList()
        ) : (
          <NotesPanel
            vimMode={vimMode}
            theme="dark"
            selectedNoteName={currentName}
            onVimStatusChange={setVimStatus}
          />
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
          {message && !error ? (
            <span className="text-green-600">{message}</span>
          ) : error ? (
            <span className="text-red-600">{error}</span>
          ) : (
            `lst ${currentView === "lists" && currentList ? `- ${currentList.title}.md` :
              currentView === "notes" && currentName ? `- ${currentName}.md` : ""}`
          )}
        </span>
        <span className="ml-auto text-nowrap flex items-center gap-2">
          {vimStatus && currentView === "notes" && (
            <span className={`px-2 py-0.5 rounded text-xs ${vimStatus.mode === "INSERT"
              ? "bg-blue-500/20 text-blue-600"
              : vimStatus.mode === "VISUAL"
                ? "bg-orange-500/20 text-orange-600"
                : "bg-green-500/20 text-green-600"
              }`}>
              {vimStatus.mode}
            </span>
          )}
          <span>
            {currentView === "lists" && currentList ? `${items.length} items` :
              currentView === "notes" && currentName ? "Note" :
                currentView === "lists" ? "No list selected" : "No note selected"}
          </span>
        </span>
      </div>
    </div>
  );
}
