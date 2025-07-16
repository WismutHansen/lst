# Testing Live Updates

## Setup
1. Start the desktop app: `cd apps/lst-desktop && bun run tauri dev`
2. In another terminal, use the CLI to make changes

## Test Cases

### Test 1: Add item to list
1. In desktop app, create or open a list (e.g., "groceries")
2. In CLI: `./target/debug/lst add groceries "milk"`
3. Expected: Desktop app should automatically refresh and show the new item

### Test 2: Mark item as done
1. In CLI: `./target/debug/lst done groceries "milk"`
2. Expected: Desktop app should show the item as completed

### Test 3: Remove item
1. In CLI: `./target/debug/lst rm groceries "milk"`
2. Expected: Desktop app should remove the item from the list

### Test 4: Create new list
1. In CLI: `./target/debug/lst new "shopping"`
2. In CLI: `./target/debug/lst add shopping "bread"`
3. Expected: Desktop app should show the new list in the sidebar and refresh if viewing it

### Test 5: Note operations
1. In CLI: `./target/debug/lst note new "test-note"`
2. In CLI: `./target/debug/lst note add "test-note" "Some content"`
3. Expected: Desktop app should show notification about note updates

## Expected Behavior
- CLI commands should send HTTP requests to `localhost:33333`
- Desktop app should receive Tauri events and refresh the UI
- No errors should appear in either CLI or desktop app console
- Updates should be near-instantaneous (within 1-2 seconds)

## Debugging
- Check desktop app console for event logs (🎧, 📨, 🔇 emojis)
- Check CLI for any HTTP request errors
- Verify command server is running on port 33333