import React, { useState, useEffect, useCallback } from "react";
import { commands, Note } from "../bindings";
import { MobileNoteEditor } from "./MobileNoteEditor";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { ScrollArea } from "./ui/scroll-area";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "./ui/sheet";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "./ui/dialog";
import { FileText, Plus, Search, Edit, Trash2, Save, ArrowLeft, Menu } from "lucide-react";

interface MobileNotesPanelProps {
  vimMode?: boolean
  theme?: "light" | "dark"
}

export function MobileNotesPanel({ vimMode = false, theme = "light" }: MobileNotesPanelProps) {
  const [notes, setNotes] = useState<string[]>([]);
  const [selectedNote, setSelectedNote] = useState<Note | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [newNoteTitle, setNewNoteTitle] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showNotesList, setShowNotesList] = useState(false);

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
        setShowNotesList(false); // Close notes list on mobile
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
        setShowNotesList(false);
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
        setShowNotesList(true); // Return to notes list
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
      } else if (selectedNote) {
        setSelectedNote(null);
        setShowNotesList(true);
      }
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

  // Mobile view: show either notes list or selected note
  if (!selectedNote || showNotesList) {
    return (
      <div className="flex flex-col h-full">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <FileText className="h-6 w-6" />
            Notes
          </h2>
          <Dialog open={isCreating} onOpenChange={setIsCreating}>
            <DialogTrigger asChild>
              <Button variant="outline" size="sm">
                <Plus className="h-4 w-4 mr-2" />
                New
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Create New Note</DialogTitle>
              </DialogHeader>
              <div className="space-y-4 mt-4">
                <Input
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
                <div className="flex gap-2">
                  <Button onClick={handleCreateNote} disabled={!newNoteTitle.trim() || loading}>
                    Create Note
                  </Button>
                  <Button variant="outline" onClick={() => setIsCreating(false)}>
                    Cancel
                  </Button>
                </div>
              </div>
            </DialogContent>
          </Dialog>
        </div>

        {/* Search */}
        <div className="p-4 border-b">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search notes..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
        </div>

        {/* Error Display */}
        {error && (
          <div className="mx-4 mt-4 text-sm text-destructive bg-destructive/10 p-3 rounded-md">
            {error}
          </div>
        )}

        {/* Notes List */}
        <ScrollArea className="flex-1">
          <div className="p-4 space-y-3">
            {filteredNotes.map((noteName) => (
              <Card
                key={noteName}
                className="cursor-pointer transition-colors hover:bg-muted active:bg-accent/80"
                onClick={() => handleSelectNote(noteName)}
              >
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <span className="font-medium truncate pr-2">
                      {noteName.replace(".md", "")}
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteNote(noteName);
                      }}
                      disabled={loading}
                      className="h-8 w-8 p-0 shrink-0"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </CardContent>
              </Card>
            ))}
            {filteredNotes.length === 0 && (
              <div className="text-center text-muted-foreground py-8">
                <FileText className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p className="text-lg mb-2">No notes found</p>
                <p className="text-sm">Create your first note to get started</p>
              </div>
            )}
          </div>
        </ScrollArea>
      </div>
    );
  }

  // Mobile view: show selected note
  return (
    <div className="flex flex-col h-full">
      {/* Note Header */}
      <div className="flex items-center justify-between p-4 border-b bg-background">
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowNotesList(true)}
            className="p-2"
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h1 className="text-lg font-semibold truncate">{selectedNote.title}</h1>
        </div>
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

      {/* Note Content */}
      <div className="flex-1 flex flex-col">
        {isEditing ? (
          <MobileNoteEditor
            value={selectedNote.content}
            onChange={handleNoteContentChange}
            vimMode={vimMode}
            theme={theme}
            onSave={handleSaveNote}
            onEscape={() => setIsEditing(false)}
            placeholder="Write your note here..."
            className="flex-1"
          />
        ) : (
          <ScrollArea className="flex-1">
            <div className="p-4">
              <div className="prose max-w-none">
                <pre className="whitespace-pre-wrap font-mono text-sm leading-relaxed">
                  {selectedNote.content || "This note is empty. Tap Edit to add content."}
                </pre>
              </div>
            </div>
          </ScrollArea>
        )}
      </div>
    </div>
  );
}
