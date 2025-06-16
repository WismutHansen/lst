
export interface ListItem {
  id: string;
  content: string;
  completed: boolean;
  createdAt: Date;
}

export interface List {
  id: string;
  name: string;
  path: string;
  items: ListItem[];
  createdAt: Date;
  modifiedAt: Date;
}

export interface Note {
  id: string;
  name: string;
  path: string;
  content: string;
  createdAt: Date;
  modifiedAt: Date;
}

export interface Directory {
  id: string;
  name: string;
  path: string;
  lists: List[];
  notes: Note[];
  subdirectories: Directory[];
}

export interface SyncStatus {
  connected: boolean;
  syncing: boolean;
  lastSync?: Date;
  error?: string;
}

export interface AppConfig {
  contentDirectory: string;
  serverUrl?: string;
  email?: string;
}
