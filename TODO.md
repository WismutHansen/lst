# TODO List for lst Project

## Core Infrastructure

- [ ] Set up Rust project structure with Cargo
- [ ] Create basic command-line interface structure
- [ ] Implement core storage model for content directories
- [ ] Design CRDT implementation for list synchronization
- [ ] Create file format parsers for lists, notes, and posts
- [ ] Implement anchor generation and tracking
- [ ] Set up Git-based three-way merge for notes/posts

## Server Components

- [ ] Build Axum API server
- [ ] Implement authentication with magic links
- [ ] Set up SMTP email delivery with lettre
- [ ] Create CRDT + Git storage backend
- [ ] Build Zola static site generation pipeline
- [ ] Configure server deployment for Proxmox LXC
- [ ] Set up reverse proxy configuration

## CLI Implementation (`lst`)

- [ ] Implement `lst ls` command
- [ ] Implement `lst add <list> <text>` command
- [ ] Implement `lst done <list> <anchor>` command 
- [ ] Implement `lst pipe <list>` command
- [ ] Implement note commands (`note new`, `note open`)
- [ ] Implement post commands (`post new`, `post list`, `post publish`)
- [ ] Add `--json` output option for all commands

## Client Applications

- [ ] Build Tauri slim GUI
  - [ ] Create toggleable, always-on-top window
  - [ ] Implement Markdown viewer/editor
  - [ ] Add sync status tray icon
- [ ] Develop Tauri 2 mobile app
  - [ ] Implement offline SQLite cache with CRDT sync
  - [ ] Add share-sheet "Add to list" functionality 
  - [ ] Create AppIntents integration
- [ ] Build Apple Shortcuts integration
  - [ ] Implement AddItem, RemoveItem intents
  - [ ] Implement GetList, DraftPost intents
- [ ] Develop AGNO voice agent
  - [ ] Integrate Whisper transcription
  - [ ] Create AGNO agent for natural language processing
  - [ ] Implement JSON action interface

## DevOps

- [ ] Create systemd service file for server
- [ ] Set up Proxmox LXC deployment scripts
- [ ] Configure DNS for email (SPF/DKIM)
- [ ] Decide on Zola deployment strategy (on-prem vs CDN)
- [ ] Implement E2E encryption
- [ ] Create invite link system
- [ ] Set up CI pipeline

## Documentation

- [ ] Write installation guide
- [ ] Create user documentation
- [ ] Document API endpoints
- [ ] Write developer documentation
- [ ] Document file formats and schemas
- [ ] Create architecture diagrams