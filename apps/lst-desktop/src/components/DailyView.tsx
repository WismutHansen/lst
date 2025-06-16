
import React from 'react';
import { List, Note } from '@/types/List';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import ListView from './ListView';
import NoteView from './NoteView';
import { Calendar } from 'lucide-react';

interface DailyViewProps {
  dailyList?: List;
  dailyNote?: Note;
  onUpdateList: (list: List) => void;
  onUpdateNote: (note: Note) => void;
}

const DailyView = ({ dailyList, dailyNote, onUpdateList, onUpdateNote }: DailyViewProps) => {
  const today = new Date().toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  });

  return (
    <div className="h-full space-y-4">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Calendar size={20} />
            Daily Workflow - {today}
          </CardTitle>
        </CardHeader>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 h-[calc(100%-5rem)]">
        <div>
          {dailyList ? (
            <ListView list={dailyList} onUpdateList={onUpdateList} />
          ) : (
            <Card className="h-full">
              <CardContent className="flex items-center justify-center h-full">
                <p className="text-muted-foreground">No daily list found (dl)</p>
              </CardContent>
            </Card>
          )}
        </div>

        <div>
          {dailyNote ? (
            <NoteView note={dailyNote} onUpdateNote={onUpdateNote} />
          ) : (
            <Card className="h-full">
              <CardContent className="flex items-center justify-center h-full">
                <p className="text-muted-foreground">No daily note found (dn)</p>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
};

export default DailyView;
