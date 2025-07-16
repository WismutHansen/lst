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
  onVimStatusChange?: (status: { mode: string; status?: string } | null) => void
}

export function VimNoteEditor({
  value,
  onChange,
  vimMode = false,
  theme = 'light',
  onSave,
  onEscape,
  placeholder = 'Start writing your note...',
  onVimStatusChange
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
        padding: '0',
        minHeight: '300px',
        backgroundColor: 'transparent'
      },
      '.cm-focused': {
        outline: 'none'
      },
      '.cm-editor': {
        border: 'none',
        backgroundColor: 'transparent'
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

  // Track vim status and notify parent
  const trackVimStatus = useCallback((view: EditorView) => {
    if (!vimMode || !onVimStatusChange) return
    
    const vimState = (view.state as any).vim
    if (vimState) {
      onVimStatusChange({
        mode: vimState.mode?.toUpperCase() || 'NORMAL',
        status: vimState.status
      })
    } else {
      onVimStatusChange(null)
    }
  }, [vimMode, onVimStatusChange])

  // Create a custom dark theme that inherits background
  const customDarkTheme = EditorView.theme({
    '&.cm-editor': {
      backgroundColor: 'transparent'
    },
    '.cm-content': {
      backgroundColor: 'transparent'
    },
    '.cm-focused': {
      backgroundColor: 'transparent'
    }
  }, { dark: true })

  // Track vim status changes
  useEffect(() => {
    if (editorRef.current?.view && vimMode) {
      const view = editorRef.current.view
      trackVimStatus(view)
      
      // Set up a listener for vim state changes
      const updateStatus = () => trackVimStatus(view)
      view.dom.addEventListener('keyup', updateStatus)
      view.dom.addEventListener('click', updateStatus)
      
      return () => {
        view.dom.removeEventListener('keyup', updateStatus)
        view.dom.removeEventListener('click', updateStatus)
      }
    }
  }, [trackVimStatus, vimMode])

  return (
    <div className="relative">
      <CodeMirror
        ref={editorRef}
        value={value}
        onChange={onChange}
        extensions={[...extensions, ...(theme === 'dark' ? [oneDark, customDarkTheme] : [])]}
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
    </div>
  )
}