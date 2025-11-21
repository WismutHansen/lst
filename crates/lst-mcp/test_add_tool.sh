#!/bin/bash
# Test add_to_list specifically

SERVER="/Users/tommyfalkowski/Code/rust/lst/target/release/lst-mcp"

echo "Building server..."
cargo build --release --quiet 2>&1 | grep -v "unused manifest key" | grep -v "function.*normalize_note" | grep -v "warn(dead_code)" | grep -v "^$" || true

echo ""
echo "=== Testing add_to_list tool ==="
(
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}'
  sleep 0.1
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_to_list","arguments":{"list":"mcp_test","item":"test item from mcp"}}}'
  sleep 0.1
) | $SERVER 2>&1 | grep -v "^Starting lst-mcp" | grep -v "^$"

echo ""
echo "=== Checking if file was created ==="
LISTS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/lst/content/lists"
if [ -f "$LISTS_DIR/mcp_test.md" ]; then
  echo "✓ File created successfully"
  echo "Contents:"
  cat "$LISTS_DIR/mcp_test.md"
  rm -f "$LISTS_DIR/mcp_test.md"
else
  echo "✗ File was not created"
  echo "Expected location: $LISTS_DIR/mcp_test.md"
  echo "Listing directory:"
  ls -la "$LISTS_DIR" | tail -5
fi
