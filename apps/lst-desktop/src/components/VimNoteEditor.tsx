import React, { useCallback, useEffect, useRef, useState } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { markdown } from '@codemirror/lang-markdown'
import { oneDark } from '@codemirror/theme-one-dark'
import { vim, Vim, getCM } from '@replit/codemirror-vim'
import { EditorView } from '@codemirror/view'
import { EditorState, Extension } from '@codemirror/state'

interface VimNoteEditorProps {
  value: string
  onChange: (value: string) => void
  vimMode?: boolean
  theme?: 'light' | 'dark'
  onSave?: () => void
  onEscape?: () => void
  placeholder?: string
}

export function VimNoteEditor({
  value,
  onChange,
  vimMode = false,
  theme = 'light',
  onSave,
  onEscape,
  placeholder = 'Start writing your note...'
}: VimNoteEditorProps) {
  const editorRef = useRef<any>(null)
  const [isVimMode, setIsVimMode] = useState(vimMode)

  const extensions: Extension[] = [
    markdown(),
    EditorView.theme({
      '&': {
        fontSize: '14px',
        fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace'
      },
      '.cm-content': {
        padding: '12px',
        minHeight: '300px'
      },
      '.cm-focused': {
        outline: 'none'
      },
      '.cm-editor': {
        borderRadius: '6px',
        border: '1px solid hsl(var(--border))'
      },
      '.cm-scroller': {
        lineHeight: '1.6'
      }
    }),
    EditorView.lineWrapping
  ]

  if (isVimMode) {
    extensions.push(vim({
      status: true
    }))
  }

  const setupVimCommands = useCallback(() => {
    if (!editorRef.current || !isVimMode) return
    
    const view = editorRef.current.view
    if (!view) return
    
    const cm = getCM(view)
    
    // Define custom ex commands
    Vim.defineEx('write', 'w', function(cm: any, input: any) {
      if (onSave) {
        onSave()
      }
    })
    
    Vim.defineEx('quit', 'q', function(cm: any, input: any) {
      if (onEscape) {
        onEscape()
      }
    })
    
    Vim.defineEx('wq', 'wq', function(cm: any, input: any) {
      if (onSave) {
        onSave()
      }
      if (onEscape) {
        onEscape()
      }
    })
    
  }, [isVimMode, onSave, onEscape])

  useEffect(() => {
    if (isVimMode) {
      setupVimCommands()
    }
  }, [setupVimCommands])

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!vimMode) return

    if (event.key === 'Escape' && onEscape) {
      onEscape()
      return
    }

    if ((event.ctrlKey || event.metaKey) && event.key === 's') {
      event.preventDefault()
      if (onSave) {
        onSave()
      }
    }
  }, [vimMode, onSave, onEscape])

  useEffect(() => {
    setIsVimMode(vimMode)
  }, [vimMode])

  useEffect(() => {
    if (editorRef.current && vimMode) {
      const editor = editorRef.current
      const view = editor.view
      
      if (view) {
        view.dom.addEventListener('keydown', handleKeyDown)
        
        return () => {
          view.dom.removeEventListener('keydown', handleKeyDown)
        }
      }
    }
  }, [handleKeyDown, vimMode])

  const vimModeStatus = useCallback((view: EditorView) => {
    if (!vimMode) return null
    
    const vimState = (view.state as any).vim
    if (!vimState) return null
    
    return (
      <div className="flex items-center gap-2 px-3 py-1 text-xs text-muted-foreground bg-muted/50 border-t">
        <span className={`px-2 py-0.5 rounded ${
          vimState.mode === 'insert' 
            ? 'bg-blue-500/20 text-blue-600' 
            : vimState.mode === 'visual'
            ? 'bg-orange-500/20 text-orange-600'
            : 'bg-green-500/20 text-green-600'
        }`}>
          {vimState.mode?.toUpperCase() || 'NORMAL'}
        </span>
        {vimState.status && (
          <span className="text-muted-foreground">{vimState.status}</span>
        )}
      </div>
    )
  }, [vimMode])

  return (
    <div className="relative">
      <CodeMirror
        ref={editorRef}
        value={value}
        onChange={onChange}
        extensions={extensions}
        theme={theme === 'dark' ? oneDark : undefined}
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
          tabSize: 2
        }}
      />
      {vimMode && editorRef.current?.view && vimModeStatus(editorRef.current.view)}
    </div>
  )
}