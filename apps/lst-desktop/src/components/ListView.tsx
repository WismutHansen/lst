
import React, { useState } from 'react';
import { List, ListItem } from '@/types/List';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Plus, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';

interface ListViewProps {
  list: List;
  onUpdateList: (list: List) => void;
}

const ListView = ({ list, onUpdateList }: ListViewProps) => {
  const [newItemContent, setNewItemContent] = useState('');

  const addItem = () => {
    if (!newItemContent.trim()) return;

    const newItem: ListItem = {
      id: crypto.randomUUID(),
      content: newItemContent.trim(),
      completed: false,
      createdAt: new Date(),
    };

    const updatedList = {
      ...list,
      items: [...list.items, newItem],
      modifiedAt: new Date(),
    };

    onUpdateList(updatedList);
    setNewItemContent('');
  };

  const toggleItem = (itemId: string) => {
    const updatedItems = list.items.map(item =>
      item.id === itemId ? { ...item, completed: !item.completed } : item
    );

    const updatedList = {
      ...list,
      items: updatedItems,
      modifiedAt: new Date(),
    };

    onUpdateList(updatedList);
  };

  const deleteItem = (itemId: string) => {
    const updatedItems = list.items.filter(item => item.id !== itemId);

    const updatedList = {
      ...list,
      items: updatedItems,
      modifiedAt: new Date(),
    };

    onUpdateList(updatedList);
  };

  const activeTasks = list.items.filter(item => !item.completed);
  const completedTasks = list.items.filter(item => item.completed);

  return (
    <Card className="h-full glass-container bg-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-card-foreground">
          {list.name}
          <span className="text-sm text-muted-foreground">
            ({activeTasks.length} active)
          </span>
        </CardTitle>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Add new item */}
        <div className="flex gap-2">
          <Input
            placeholder="Add new item..."
            value={newItemContent}
            onChange={(e) => setNewItemContent(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && addItem()}
            className="flex-1 glass-light bg-input border-border"
          />
          <Button onClick={addItem} size="sm" className="glass-light bg-primary text-primary-foreground hover:bg-primary/90">
            <Plus size={16} />
          </Button>
        </div>

        {/* Active items */}
        {activeTasks.length > 0 && (
          <div className="space-y-2">
            <h4 className="text-sm text-muted-foreground">Active</h4>
            {activeTasks.map((item) => (
              <div key={item.id} className="flex items-center gap-3 p-3 glass-item">
                <Checkbox
                  checked={item.completed}
                  onCheckedChange={() => toggleItem(item.id)}
                />
                <span className="flex-1">{item.content}</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => deleteItem(item.id)}
                  className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                >
                  <Trash2 size={14} />
                </Button>
              </div>
            ))}
          </div>
        )}

        {/* Completed items */}
        {completedTasks.length > 0 && (
          <div className="space-y-2">
            <h4 className="text-sm  text-muted-foreground">Completed</h4>
            {completedTasks.map((item) => (
              <div key={item.id} className="flex items-center gap-3 p-2 rounded-md hover:bg-accent/50">
                <Checkbox
                  checked={item.completed}
                  onCheckedChange={() => toggleItem(item.id)}
                />
                <span className={cn("flex-1", item.completed && "line-through text-muted-foreground")}>
                  {item.content}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => deleteItem(item.id)}
                  className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                >
                  <Trash2 size={14} />
                </Button>
              </div>
            ))}
          </div>
        )}

        {list.items.length === 0 && (
          <div className="text-center py-8 text-muted-foreground">
            No items yet. Add one above to get started.
          </div>
        )}
      </CardContent>
    </Card>
  );
};

export default ListView;
