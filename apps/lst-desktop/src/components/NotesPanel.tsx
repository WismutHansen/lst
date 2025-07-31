import React, { useState, useEffect, useCallback } from "react";
import { commands, Note } from "../bindings";
import { VimNoteEditor } from "./VimNoteEditor";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { ScrollArea } from "./ui/scroll-area";
import { Separator } from "./ui/separator";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "./ui/sheet";
import { FileText, Plus, Search, Edit, Trash2, Save } from "lucide-react";

interface NotesPanelProps {
  vimMode?: boolean
  theme?: "light" | "dark"
  selectedNoteName?: string | null
  onVimStatusChange?: (status: { mode: string; status?: string } | null) => void
}

export function NotesPanel({ vimMode = false, theme = "light", selectedNoteName = null, onVimStatusChange }: NotesPanelProps) {
  const [notes, setNotes] = useState<string[]>([]);
  const [selectedNote, setSelectedNote] = useState<Note | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [newNoteTitle, setNewNoteTitle] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadNotes = useCallback(async () => {
    try {
      const result = await commands.getNotes();
      if (result.status === "ok") {
        setNotes(result.data);
      } else {
        setError(result.error);
      }
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    loadNotes();
  }, [loadNotes]);

  useEffect(() => {
    if (selectedNoteName) {
      handleSelectNote(selectedNoteName);
    }
  }, [selectedNoteName]);

  const filteredNotes = notes.filter(note =>
    note.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleSelectNote = async (noteName: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await commands.getNote(noteName);
      if (result.status === "ok") {
        setSelectedNote(result.data);
        setIsEditing(false);
        onVimStatusChange?.(null); // Clear vim status when switching notes
      } else {
        setError(result.error);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleCreateNote = async () => {
    if (!newNoteTitle.trim()) return;

    setLoading(true);
    setError(null);
    try {
      const result = await commands.createNoteCmd(newNoteTitle.trim());
      if (result.status === "ok") {
        setSelectedNote(result.data);
        setIsEditing(true);
        setIsCreating(false);
        setNewNoteTitle("");
        await loadNotes();
      } else {
        setError(result.error);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSaveNote = async () => {
    if (!selectedNote) return;

    setLoading(true);
    setError(null);
    try {
      const result = await commands.saveNote(selectedNote);
      if (result.status === "ok") {
        setIsEditing(false);
        onVimStatusChange?.(null); // Clear vim status when exiting edit mode
      } else {
        setError(result.error);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteNote = async (noteName: string) => {
    if (!confirm(`Are you sure you want to delete "${noteName}"?`)) return;

    setLoading(true);
    setError(null);
    try {
      const result = await commands.deleteNoteCmd(noteName);
      if (result.status === "ok") {
        if (selectedNote && selectedNote.title === noteName) {
          setSelectedNote(null);
          setIsEditing(false);
        }
        await loadNotes();
      } else {
        setError(result.error);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleNoteContentChange = (content: string) => {
    if (selectedNote) {
      setSelectedNote({ ...selectedNote, content });
    }
  };

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!vimMode) return;

    if (event.key === "Escape") {
      if (isEditing) {
        setIsEditing(false);
        onVimStatusChange?.(null); // Clear vim status when exiting edit mode
      } else if (selectedNote) {
        setSelectedNote(null);
        onVimStatusChange?.(null); // Clear vim status when closing note
      }
      return;
    }

    if (event.key === "i" && !isEditing && selectedNote) {
      event.preventDefault();
      setIsEditing(true);
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key === "s") {
      event.preventDefault();
      if (isEditing && selectedNote) {
        handleSaveNote();
      }
    }
  }, [vimMode, isEditing, selectedNote]);

  useEffect(() => {
    if (vimMode) {
      document.addEventListener("keydown", handleKeyDown);
      return () => document.removeEventListener("keydown", handleKeyDown);
    }
  }, [handleKeyDown, vimMode]);

  return (
    <div className="flex h-full w-full">
      {/* Note Content Area */}
      <div className="flex-1 flex flex-col">
        {selectedNote ? (
          <>
            <div className="border-b p-4 flex items-center justify-between bg-background">
              <h1 className="text-xl font-semibold">{selectedNote.title}</h1>
              <div className="flex items-center gap-2">
                {isEditing ? (
                  <>
                    <Button onClick={handleSaveNote} disabled={loading} size="sm">
                      <Save className="h-4 w-4 mr-2" />
                      Save
                    </Button>
                    <Button
                      variant="outline"
                      onClick={() => setIsEditing(false)}
                      size="sm"
                    >
                      Cancel
                    </Button>
                  </>
                ) : (
                  <Button
                    onClick={() => setIsEditing(true)}
                    disabled={loading}
                    size="sm"
                  >
                    <Edit className="h-4 w-4 mr-2" />
                    Edit
                  </Button>
                )}
              </div>
            </div>

            <div className="flex-1 p-4">
              {isEditing ? (
                <VimNoteEditor
                  value={selectedNote.content}
                  onChange={handleNoteContentChange}
                  vimMode={vimMode}
                  theme={theme}
                  onSave={handleSaveNote}
                  onEscape={() => setIsEditing(false)}
                  placeholder="Write your note here..."
                  onVimStatusChange={onVimStatusChange}
                />
              ) : (
                <div 
                  className="cursor-text min-h-[300px]"
                  onClick={() => setIsEditing(true)}
                  style={{
                    fontSize: "14px",
                    fontFamily: "ui-monospace, SFMono-Regular, \"SF Mono\", Monaco, \"Cascadia Code\", \"Roboto Mono\", Consolas, \"Courier New\", monospace",
                    lineHeight: "1.6"
                  }}
                >
                  <pre className="whitespace-pre-wrap m-0 p-0 bg-transparent border-none">
                    {selectedNote.content || "This note is empty. Click here or press i to add content."}
                  </pre>
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <FileText className="h-16 w-16 mx-auto mb-4 opacity-50" />
              <p className="text-lg mb-2">No note selected</p>
              <p className="text-sm">Choose a note from the sidebar or create a new one</p>
            </div>
          </div>
        )}
      </div>

      {/* Create Note Dialog */}
      <Sheet open={isCreating} onOpenChange={setIsCreating}>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>Create New Note</SheetTitle>
          </SheetHeader>
          <div className="space-y-4 mt-6">
            <div>
              <label htmlFor="note-title" className="text-sm font-medium">
                Note Title
              </label>
              <Input
                id="note-title"
                value={newNoteTitle}
                onChange={(e) => setNewNoteTitle(e.target.value)}
                placeholder="Enter note title..."
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleCreateNote();
                  }
                }}
                autoFocus
              />
            </div>
            <div className="flex gap-2">
              <Button onClick={handleCreateNote} disabled={!newNoteTitle.trim() || loading}>
                Create Note
              </Button>
              <Button variant="outline" onClick={() => setIsCreating(false)}>
                Cancel
              </Button>
            </div>
          </div>
        </SheetContent>
      </Sheet>
    </div>
  );
}