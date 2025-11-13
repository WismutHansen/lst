#!/usr/bin/env python3
"""
Test client for lst-mcp server
This script provides an interactive way to test the MCP server
"""

import json
import subprocess
import sys
from typing import Any, Dict, Optional


class MCPClient:
    """Simple MCP client for testing"""

    def __init__(self, server_command: list[str]):
        """Initialize the client with server command"""
        self.server_command = server_command
        self.process: Optional[subprocess.Popen] = None
        self.request_id = 0

    def start(self):
        """Start the MCP server process"""
        import os
        print(f"Starting server: {' '.join(self.server_command)}")
        
        # Suppress tracing logs by not setting RUST_LOG
        env = os.environ.copy()
        env.pop('RUST_LOG', None)  # Remove any RUST_LOG setting
        
        self.process = subprocess.Popen(
            self.server_command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        print("Server started")
        
        # Give server a moment to initialize
        import time
        time.sleep(0.5)

    def stop(self):
        """Stop the MCP server process"""
        if self.process:
            self.process.terminate()
            self.process.wait()
            print("Server stopped")

    def send_request(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Send a JSON-RPC request to the server"""
        if not self.process or not self.process.stdin or not self.process.stdout:
            raise RuntimeError("Server not started properly")

        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params,
        }

        print(f"\n→ Request: {json.dumps(request, indent=2)}")

        # Send request
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()

        # Read response
        response_line = self.process.stdout.readline()
        if not response_line or not response_line.strip():
            # Try to get stderr for debugging
            import select
            if self.process.stderr:
                # Non-blocking read of stderr
                stderr_lines = []
                while True:
                    ready, _, _ = select.select([self.process.stderr], [], [], 0.1)
                    if ready:
                        line = self.process.stderr.readline()
                        if line:
                            stderr_lines.append(line)
                        else:
                            break
                    else:
                        break
                stderr = "".join(stderr_lines)
            else:
                stderr = "No stderr available"
            raise RuntimeError(f"No response from server. Response line: '{response_line}'. Stderr: {stderr}")

        try:
            response = json.loads(response_line.strip())
            print(f"← Response: {json.dumps(response, indent=2)}")
            return response
        except json.JSONDecodeError as e:
            raise RuntimeError(f"Failed to parse JSON response: {response_line}. Error: {e}")

    def initialize(self):
        """Initialize the MCP server"""
        return self.send_request(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0.1.0"},
            },
        )

    def list_tools(self):
        """List available tools"""
        return self.send_request("tools/list", {})

    def call_tool(self, name: str, arguments: Dict[str, Any]):
        """Call a tool"""
        return self.send_request("tools/call", {"name": name, "arguments": arguments})


def run_tests():
    """Run a series of tests against the MCP server"""
    # Build the server first
    print("Building server...")
    subprocess.run(
        ["cargo", "build", "--release"],
        cwd="/Users/tommyfalkowski/Code/rust/lst/crates/lst-mcp",
        check=True,
    )

    client = MCPClient(
        [
            "/Users/tommyfalkowski/Code/rust/lst/target/release/lst-mcp",
        ]
    )

    try:
        client.start()

        # Test 1: Initialize
        print("\n" + "=" * 60)
        print("Test 1: Initialize")
        print("=" * 60)
        response = client.initialize()
        assert "result" in response
        print("✓ Initialize successful")

        # Test 2: List tools
        print("\n" + "=" * 60)
        print("Test 2: List Tools")
        print("=" * 60)
        response = client.list_tools()
        assert "result" in response
        tools = response["result"]["tools"]
        print(f"✓ Found {len(tools)} tools:")
        for tool in tools:
            print(f"  - {tool['name']}: {tool['description']}")

        # Test 3: List lists (should be empty)
        print("\n" + "=" * 60)
        print("Test 3: List Lists (empty)")
        print("=" * 60)
        response = client.call_tool("list_lists", {})
        assert "result" in response
        print("✓ List lists successful")

        # Test 4: Add item to list
        print("\n" + "=" * 60)
        print("Test 4: Add Item to List")
        print("=" * 60)
        response = client.call_tool(
            "add_to_list", {"list": "groceries", "item": "milk"}
        )
        assert "result" in response
        print("✓ Add item successful")

        # Test 5: List lists (should now have groceries)
        print("\n" + "=" * 60)
        print("Test 5: List Lists (with items)")
        print("=" * 60)
        response = client.call_tool("list_lists", {})
        assert "result" in response
        print("✓ List lists successful")

        # Test 6: Add multiple items
        print("\n" + "=" * 60)
        print("Test 6: Add Multiple Items")
        print("=" * 60)
        response = client.call_tool(
            "add_to_list", {"list": "groceries", "item": "bread, eggs, cheese"}
        )
        assert "result" in response
        print("✓ Add multiple items successful")

        # Test 7: Mark item done
        print("\n" + "=" * 60)
        print("Test 7: Mark Item Done")
        print("=" * 60)
        response = client.call_tool(
            "mark_done", {"list": "groceries", "target": "milk"}
        )
        assert "result" in response
        print("✓ Mark item done successful")

        # Test 8: Mark item undone
        print("\n" + "=" * 60)
        print("Test 8: Mark Item Undone")
        print("=" * 60)
        response = client.call_tool(
            "mark_undone", {"list": "groceries", "target": "milk"}
        )
        assert "result" in response
        print("✓ Mark item undone successful")

        # Test 9: Error handling - nonexistent list
        print("\n" + "=" * 60)
        print("Test 9: Error Handling - Nonexistent List")
        print("=" * 60)
        response = client.call_tool(
            "mark_done", {"list": "nonexistent", "target": "item"}
        )
        if "error" in response:
            print("✓ Error handling works correctly")
        else:
            print("✗ Expected error but got success")

        print("\n" + "=" * 60)
        print("All tests completed!")
        print("=" * 60)

    except Exception as e:
        print(f"\n✗ Test failed: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)
    finally:
        client.stop()


if __name__ == "__main__":
    run_tests()
