# Testing lst-mcp Server

This document describes how to test and debug the lst-mcp MCP server.

## Running Unit Tests

Run the comprehensive unit test suite:

```bash
cargo test
```

Run tests with detailed output:

```bash
cargo test -- --nocapture
```

Run a specific test:

```bash
cargo test test_list_lists_empty -- --nocapture
```

## Running Integration Tests

### Shell Script Tests

For quick integration testing with real JSON-RPC protocol:

```bash
./simple_test.sh
```

This runs end-to-end tests:
- Initializes the MCP server
- Lists available tools
- Adds items to lists
- Lists all lists
- Uses your actual configured directories from config.toml
- Server logs go to stderr (so they don't interfere with JSON-RPC on stdout)

## Debugging

### Enable Logging

The server uses tracing for logging. Set the log level with:

```bash
RUST_LOG=debug cargo run
```

Available log levels:
- `RUST_LOG=error` - Only errors
- `RUST_LOG=warn` - Warnings and errors
- `RUST_LOG=info` - Info, warnings, and errors (default)
- `RUST_LOG=debug` - Debug info and above
- `RUST_LOG=trace` - All logging

### Module-specific Logging

```bash
RUST_LOG=lst_mcp=debug cargo run
```

### Test with Custom Content Directory

```bash
LST_CONTENT_DIR=/tmp/test-content cargo run
```

### Manual Testing with stdio

Start the server:

```bash
cargo run
```

Send JSON-RPC requests via stdin (one per line):

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_lists","arguments":{}}}
```

```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"add_to_list","arguments":{"list":"test","item":"item1"}}}
```

## Test Coverage

The test suite covers:

### Unit Tests
- `test_list_lists_empty` - Listing when no lists exist
- `test_list_lists_with_items` - Listing existing lists
- `test_add_to_list_new_list` - Creating a new list
- `test_add_to_list_existing_list` - Adding to existing list
- `test_add_multiple_items` - Adding comma-separated items
- `test_mark_done` - Marking items as done
- `test_mark_undone` - Marking items as undone
- `test_mark_done_nonexistent_list` - Error handling for missing lists
- `test_mark_done_nonexistent_item` - Error handling for missing items
- `test_add_to_list_with_special_characters` - Special character handling
- `test_list_lists_tool_serialization` - JSON serialization
- `test_add_to_list_tool_serialization` - JSON round-trip

### Integration Tests (Python)
- Server initialization
- Tool discovery
- All CRUD operations
- Error handling
- Multi-item operations

## Common Issues

### Server doesn't start
- Check that the binary is built: `cargo build`
- Check logs: `RUST_LOG=debug cargo run`

### Tests fail with "No lists found"
- Ensure LST_CONTENT_DIR is set correctly
- Check file permissions in content directory

### JSON parsing errors
- Verify JSON format with `jq`: `echo '{"test":1}' | jq .`
- Check for proper newlines between requests

### MCP client can't connect
- Verify server is using stdio transport (not HTTP)
- Check that requests/responses are newline-delimited JSON

## Debugging Tips

1. Run with debug logging to see all requests/responses
2. Use `test_client.py` for automated testing
3. Check the content directory after operations to verify file changes
4. Use `jq` to format and validate JSON
5. Test error cases to ensure proper error handling

## Performance Testing

To test performance with many lists:

```bash
# Create 100 test lists
for i in {1..100}; do
    echo "- [ ] item1" > content/lists/list$i.md
done

# Time the list operation
time cargo run <<< '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_lists","arguments":{}}}'
```

## CI/CD Integration

The tests can be run in CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Run tests
  run: |
    cargo test --all-features
    cargo test --release
```
