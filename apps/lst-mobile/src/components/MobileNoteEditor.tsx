import React, { useCallback, useEffect, useRef, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { vim } from "@replit/codemirror-vim";
import { EditorView } from "@codemirror/view";
import { EditorState, Extension } from "@codemirror/state";

interface MobileNoteEditorProps {
  value: string
  onChange: (value: string) => void
  vimMode?: boolean
  theme?: "light" | "dark"
  onSave?: () => void
  onEscape?: () => void
  placeholder?: string
  className?: string
}

export function MobileNoteEditor({
  value,
  onChange,
  vimMode = false,
  theme = "light",
  onSave,
  onEscape,
  placeholder = "Start writing your note...",
  className = ""
}: MobileNoteEditorProps) {
  const editorRef = useRef<any>(null);
  const [isVimMode, setIsVimMode] = useState(vimMode);

  const extensions: Extension[] = [
    markdown(),
    EditorView.theme({
      "&": {
        fontSize: "16px", // Larger font for mobile
        fontFamily: "ui-monospace, SFMono-Regular, \"SF Mono\", Monaco, \"Cascadia Code\", \"Roboto Mono\", Consolas, \"Courier New\", monospace"
      },
      ".cm-content": {
        padding: "16px", // More padding for touch
        minHeight: "200px",
        lineHeight: "1.6"
      },
      ".cm-focused": {
        outline: "none"
      },
      ".cm-editor": {
        borderRadius: "8px",
        border: "1px solid hsl(var(--border))"
      },
      ".cm-scroller": {
        lineHeight: "1.6"
      },
      // Mobile-specific touch improvements
      ".cm-cursor": {
        borderWidth: "2px"
      },
      ".cm-line": {
        padding: "2px 0"
      }
    }),
    EditorView.lineWrapping,
    // Better mobile editing experience
    EditorView.domEventHandlers({
      touchstart: () => {
        // Ensure editor is focused on touch
        return false;
      }
    })
  ];

  if (isVimMode) {
    extensions.push(vim({
      status: true
    }));
  }

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!vimMode) return;

    if (event.key === "Escape" && onEscape) {
      onEscape();
      return;
    }

    if ((event.ctrlKey || event.metaKey) && event.key === "s") {
      event.preventDefault();
      if (onSave) {
        onSave();
      }
    }
  }, [vimMode, onSave, onEscape]);

  useEffect(() => {
    setIsVimMode(vimMode);
  }, [vimMode]);

  useEffect(() => {
    if (editorRef.current && vimMode) {
      const editor = editorRef.current;
      const view = editor.view;
      
      if (view) {
        view.dom.addEventListener("keydown", handleKeyDown);
        
        return () => {
          view.dom.removeEventListener("keydown", handleKeyDown);
        };
      }
    }
  }, [handleKeyDown, vimMode]);

  const vimModeStatus = useCallback((view: EditorView) => {
    if (!vimMode) return null;
    
    const vimState = (view.state as any).vim;
    if (!vimState) return null;
    
    return (
      <div className="flex items-center gap-2 px-4 py-2 text-sm text-muted-foreground bg-muted/50 border-t">
        <span className={`px-3 py-1 rounded-full text-xs ${
          vimState.mode === "insert" 
            ? "bg-blue-500/20 text-blue-600" 
            : vimState.mode === "visual"
            ? "bg-orange-500/20 text-orange-600"
            : "bg-primary/20 text-primary"
        }`}>
          {vimState.mode?.toUpperCase() || "NORMAL"}
        </span>
        {vimState.status && (
          <span className="text-muted-foreground text-xs">{vimState.status}</span>
        )}
      </div>
    );
  }, [vimMode]);

  return (
    <div className={`relative h-full ${className}`}>
      <CodeMirror
        ref={editorRef}
        value={value}
        onChange={onChange}
        extensions={extensions}
        theme={theme === "dark" ? oneDark : undefined}
        placeholder={placeholder}
        basicSetup={{
          lineNumbers: false,
          foldGutter: false,
          dropCursor: false,
          allowMultipleSelections: false,
          indentOnInput: true,
          bracketMatching: true,
          closeBrackets: true,
          searchKeymap: true,
          tabSize: 2,
          // Mobile-optimized settings
          autocompletion: true,
          highlightSelectionMatches: false
        }}
        className="h-full"
      />
      {vimMode && editorRef.current?.view && vimModeStatus(editorRef.current.view)}
    </div>
  );
}