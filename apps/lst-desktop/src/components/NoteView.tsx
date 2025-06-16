
import React, { useState } from 'react';
import { Note } from '@/types/List';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Edit3, Eye, Save } from 'lucide-react';

interface NoteViewProps {
  note: Note;
  onUpdateNote: (note: Note) => void;
}

const NoteView = ({ note, onUpdateNote }: NoteViewProps) => {
  const [isEditing, setIsEditing] = useState(false);
  const [content, setContent] = useState(note.content);

  const saveNote = () => {
    const updatedNote = {
      ...note,
      content,
      modifiedAt: new Date(),
    };
    onUpdateNote(updatedNote);
    setIsEditing(false);
  };

  const cancelEdit = () => {
    setContent(note.content);
    setIsEditing(false);
  };

  // Simple markdown-to-html conversion for display
  const renderMarkdown = (text: string) => {
    return text
      .split('\n')
      .map((line, index) => {
        if (line.startsWith('# ')) {
          return <h1 key={index} className="text-2xl mb-2">{line.slice(2)}</h1>;
        }
        if (line.startsWith('## ')) {
          return <h2 key={index} className="text-xl mb-2">{line.slice(3)}</h2>;
        }
        if (line.startsWith('### ')) {
          return <h3 key={index} className="text-lg mb-2">{line.slice(4)}</h3>;
        }
        if (line.trim() === '') {
          return <br key={index} />;
        }
        return <p key={index} className="mb-2">{line}</p>;
      });
  };

  return (
    <Card className="h-full">
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{note.name}</CardTitle>
        <div className="flex gap-2">
          {isEditing ? (
            <>
              <Button variant="outline" size="sm" onClick={cancelEdit}>
                Cancel
              </Button>
              <Button size="sm" onClick={saveNote}>
                <Save size={16} className="mr-1" />
                Save
              </Button>
            </>
          ) : (
            <Button variant="outline" size="sm" onClick={() => setIsEditing(true)}>
              <Edit3 size={16} className="mr-1" />
              Edit
            </Button>
          )}
        </div>
      </CardHeader>

      <CardContent>
        {isEditing ? (
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="Write in Markdown..."
            className="min-h-[400px] font-mono"
          />
        ) : (
          <div className="prose prose-sm max-w-none">
            {note.content ? renderMarkdown(note.content) : (
              <p className="text-muted-foreground italic">No content yet. Click Edit to add some.</p>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
};

export default NoteView;
