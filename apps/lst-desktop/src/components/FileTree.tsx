
import React, { useState } from 'react';
import { Directory, List, Note } from '@/types/List';
import { ChevronRight, ChevronDown, FileText, List as ListIcon, Folder, FolderOpen } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

interface FileTreeProps {
  directory: Directory;
  onSelectList: (list: List) => void;
  onSelectNote: (note: Note) => void;
  selectedId?: string;
  level?: number;
}

const FileTree = ({ directory, onSelectList, onSelectNote, selectedId, level = 0 }: FileTreeProps) => {
  const [isExpanded, setIsExpanded] = useState(level < 2);

  const hasContent = directory.lists.length > 0 || directory.notes.length > 0 || directory.subdirectories.length > 0;

  return (
    <div className="select-none">
      <div className="flex items-center gap-1 py-1">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setIsExpanded(!isExpanded)}
          className="h-6 w-6 p-0"
          disabled={!hasContent}
        >
          {hasContent ? (
            isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />
          ) : null}
        </Button>

        <div className="flex items-center gap-2">
          {isExpanded ? <FolderOpen size={16} /> : <Folder size={16} />}
          <span className="text-sm">{directory.name}</span>
        </div>
      </div>

      {isExpanded && (
        <div className="ml-4 border-l border-border pl-2">
          {/* Lists */}
          {directory.lists.map((list) => (
            <Button
              key={list.id}
              variant="ghost"
              size="sm"
              onClick={() => onSelectList(list)}
              className={cn(
                "w-full justify-start gap-2 h-7 glass-item mb-1",
                selectedId === list.id && "selected bg-accent text-accent-foreground"
              )}
            >
              <ListIcon size={14} />
              <span className="truncate">{list.name}</span>
              <span className="ml-auto text-xs text-muted-foreground">
                {list.items.filter(item => !item.completed).length}
              </span>
            </Button>
          ))}

          {/* Notes */}
          {directory.notes.map((note) => (
            <Button
              key={note.id}
              variant="ghost"
              size="sm"
              onClick={() => onSelectNote(note)}
              className={cn(
                "w-full justify-start gap-2 h-7 glass-item mb-1",
                selectedId === note.id && "selected bg-accent text-accent-foreground"
              )}
            >
              <FileText size={14} />
              <span className="truncate">{note.name}</span>
            </Button>
          ))}

          {/* Subdirectories */}
          {directory.subdirectories.map((subdir) => (
            <FileTree
              key={subdir.id}
              directory={subdir}
              onSelectList={onSelectList}
              onSelectNote={onSelectNote}
              selectedId={selectedId}
              level={level + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export default FileTree;
