
import React, { useState, useEffect } from 'react';
import { Directory, List, Note, SyncStatus as SyncStatusType, AppConfig } from '@/types/List';
import FileTree from '@/components/FileTree';
import ListView from '@/components/ListView';
import NoteView from '@/components/NoteView';
import DailyView from '@/components/DailyView';
import SyncStatus from '@/components/SyncStatus';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent } from '@/components/ui/card';
import { Settings, Plus, Home } from 'lucide-react';

// Mock data for development
const mockDirectory: Directory = {
  id: 'root',
  name: 'lst',
  path: '/',
  lists: [
    {
      id: 'dl',
      name: 'dl',
      path: '/dl.md',
      items: [
        { id: '1', content: 'Review project requirements', completed: false, createdAt: new Date() },
        { id: '2', content: 'Update documentation', completed: true, createdAt: new Date() },
      ],
      createdAt: new Date(),
      modifiedAt: new Date(),
    }
  ],
  notes: [
    {
      id: 'dn',
      name: 'dn',
      path: '/dn.md',
      content: '# Daily Notes\n\n## Today\'s Focus\n\nWorking on the lst frontend application.\n\n## Key Points\n\n- Built file tree navigation\n- Implemented list and note views\n- Added daily workflow support',
      createdAt: new Date(),
      modifiedAt: new Date(),
    }
  ],
  subdirectories: [
    {
      id: 'projects',
      name: 'projects',
      path: '/projects',
      lists: [
        {
          id: 'project-a',
          name: 'Project A',
          path: '/projects/project-a.md',
          items: [
            { id: 'pa1', content: 'Research phase', completed: true, createdAt: new Date() },
            { id: 'pa2', content: 'Design mockups', completed: false, createdAt: new Date() },
          ],
          createdAt: new Date(),
          modifiedAt: new Date(),
        }
      ],
      notes: [],
      subdirectories: [],
    }
  ],
};

const mockSyncStatus: SyncStatusType = {
  connected: true,
  syncing: false,
  lastSync: new Date(),
};

const Index = () => {
  const [directory, setDirectory] = useState<Directory>(mockDirectory);
  const [selectedList, setSelectedList] = useState<List | null>(null);
  const [selectedNote, setSelectedNote] = useState<Note | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatusType>(mockSyncStatus);
  const [activeTab, setActiveTab] = useState('browse');

  // Find daily list and note
  const dailyList = directory.lists.find(list => list.name === 'dl');
  const dailyNote = directory.notes.find(note => note.name === 'dn');

  const updateList = (updatedList: List) => {
    const updateInDirectory = (dir: Directory): Directory => {
      return {
        ...dir,
        lists: dir.lists.map(list => list.id === updatedList.id ? updatedList : list),
        subdirectories: dir.subdirectories.map(updateInDirectory),
      };
    };

    setDirectory(updateInDirectory(directory));
    if (selectedList?.id === updatedList.id) {
      setSelectedList(updatedList);
    }
  };

  const updateNote = (updatedNote: Note) => {
    const updateInDirectory = (dir: Directory): Directory => {
      return {
        ...dir,
        notes: dir.notes.map(note => note.id === updatedNote.id ? updatedNote : note),
        subdirectories: dir.subdirectories.map(updateInDirectory),
      };
    };

    setDirectory(updateInDirectory(directory));
    if (selectedNote?.id === updatedNote.id) {
      setSelectedNote(updatedNote);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-transparent rounded-md">
      {/* Header */}
      <header className="glass-container px-4 py-3 flex items-center justify-between m-2 mb-0">
        <div className="flex items-center gap-4">
          <h1 className="text-xl text-foreground">lst</h1>
          <SyncStatus status={syncStatus} />
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" className="glass-light bg-input border-border hover:bg-accent">
            <Plus size={16} className="mr-1" />
            New
          </Button>
          <Button variant="ghost" size="sm" className="hover:bg-accent">
            <Settings size={16} />
          </Button>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex m-2 mt-2 gap-2">
        {/* Sidebar */}
        <div className="w-64 glass-container bg-sidebar-background p-4">
          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-2 glass-light bg-muted">
              <TabsTrigger value="browse" className="data-[state=active]:bg-accent data-[state=active]:text-accent-foreground">Browse</TabsTrigger>
              <TabsTrigger value="daily" className="data-[state=active]:bg-accent data-[state=active]:text-accent-foreground">Daily</TabsTrigger>
            </TabsList>

            <TabsContent value="browse" className="mt-4">
              <FileTree
                directory={directory}
                onSelectList={setSelectedList}
                onSelectNote={setSelectedNote}
                selectedId={selectedList?.id || selectedNote?.id}
              />
            </TabsContent>

            <TabsContent value="daily" className="mt-4">
              <Button
                variant="ghost"
                className="w-full justify-start gap-2 glass-item hover:bg-accent text-left"
                onClick={() => setActiveTab('daily-view')}
              >
                <Home size={16} />
                Today's Workflow
              </Button>
            </TabsContent>
          </Tabs>
        </div>

        {/* Content Area */}
        <div className="flex-1">
          {activeTab === 'daily-view' ? (
            <DailyView
              dailyList={dailyList}
              dailyNote={dailyNote}
              onUpdateList={updateList}
              onUpdateNote={updateNote}
            />
          ) : selectedList ? (
            <ListView list={selectedList} onUpdateList={updateList} />
          ) : selectedNote ? (
            <NoteView note={selectedNote} onUpdateNote={updateNote} />
          ) : (
            <Card className="h-full glass-container bg-card">
              <CardContent className="flex items-center justify-center h-full">
                <div className="text-center">
                  <p className="text-muted-foreground mb-4">
                    Select a list or note from the sidebar to get started
                  </p>
                  <Button onClick={() => setActiveTab('daily')} className="glass-light bg-primary text-primary-foreground hover:bg-primary/90">
                    <Home size={16} className="mr-2" />
                    View Daily Workflow
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
};

export default Index;
