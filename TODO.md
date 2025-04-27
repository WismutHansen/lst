# TODO List for lst Project

## Core Infrastructure

- [x] Set up Rust project structure with Cargo
- [x] Create basic command-line interface structure
- [x] Implement core storage model for content directories
- [x] Create file format parsers for lists
- [x] Implement anchor generation and tracking
- [ ] Design CRDT implementation for list synchronization
- [ ] Create file format parsers for notes and posts
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

- [x] Implement `lst ls` command
- [x] Implement `lst add <list> <text>` command
- [x] Implement `lst done <list> <target>` command with fuzzy matching
- [x] Implement `lst pipe <list>` command
- [x] Add `--json` output option for all commands
- [x] Implement note commands (`note new`, `note add`, `note open`)
- [ ] Implement post commands (`post new`, `post list`, `post publish`)
- [ ] Implement image commands (`img add`, `img paste`, `img list`, `img rm`)

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

## Next Immediate Tasks

 - [x] Implement configuration loading from `~/.config/lst/lst.toml`
 - [x] Implement note commands
 - [ ] Improve error handling and user feedback
 - [ ] Add tests for core functionality
 - [ ] Implement image support with Git LFS

## DevOps

- [ ] Create systemd service file for server
- [ ] Set up Proxmox LXC deployment scripts
- [ ] Configure DNS for email (SPF/DKIM)
- [ ] Decide on Zola deployment strategy (on-prem vs CDN)
- [ ] Implement E2E encryption
- [ ] Create invite link system
- [ ] Set up CI pipeline

## Documentation

- [x] Create initial SPEC.md
- [ ] Write installation guide
- [ ] Create user documentation
- [ ] Document API endpoints
- [ ] Write developer documentation
- [ ] Document file formats and schemas
- [ ] Create architecture diagrams