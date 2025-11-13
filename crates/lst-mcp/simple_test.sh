#!/bin/bash
# Simple test script for lst-mcp server

set -e

SERVER="/Users/tommyfalkowski/Code/rust/lst/target/release/lst-mcp"

echo "Building server..."
cargo build --release --quiet

echo ""
echo "=== Test 1: Initialize ==="
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' |
  $SERVER 2>/dev/null | jq .

echo ""
echo "=== Test 2: List Tools ==="
(
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
) | $SERVER 2>/dev/null | tail -1 | jq '.result.tools[] | {name, description}'

echo ""
echo "=== Test 3: Add Item ==="
(
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_to_list","arguments":{"list":"test_simple","item":"test item"}}}'
) | $SERVER 2>/dev/null | tail -1 | jq .

echo ""
echo "=== Test 4: List Lists ==="
(
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_lists","arguments":{}}}'
) | $SERVER 2>/dev/null | tail -1 | jq .

echo ""
echo "=== All tests completed successfully! ==="

# Clean up
rm -f ~/.local/share/lst/content/lists/test_simple.md || echo "no files found"
