#!/usr/bin/env python3
"""
Test script to verify the MCP server works properly.
This script sends JSON-RPC messages to the MCP server via stdin/stdout.
"""

import json
import subprocess
import sys

def test_mcp_server():
    """Test the MCP server by sending JSON-RPC messages."""
    
    # Start the MCP server process
    proc = subprocess.Popen(
        ['./target/debug/codegraph', 'serve', '--index', 'test_project/.codegraph/index.bin'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    def send_request(method, params=None, request_id=1):
        """Send a JSON-RPC request and get response."""
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "id": request_id
        }
        if params:
            request["params"] = params
        
        request_json = json.dumps(request)
        print(f"📤 Sending: {request_json}")
        
        proc.stdin.write(request_json + "\n")
        proc.stdin.flush()
        
        response_line = proc.stdout.readline().strip()
        print(f"📥 Received: {response_line}")
        
        if response_line:
            try:
                return json.loads(response_line)
            except json.JSONDecodeError as e:
                print(f"❌ Failed to parse response: {e}")
                return None
        return None
    
    try:
        # Test 1: Initialize the server
        print("🔧 Testing initialization...")
        init_response = send_request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }, 1)
        
        if init_response and init_response.get("result"):
            print("✅ Initialization successful")
        else:
            print("❌ Initialization failed")
            return False
        
        # Test 2: List tools
        print("\n🛠️ Testing tools/list...")
        tools_response = send_request("tools/list", {}, 2)
        
        if tools_response and tools_response.get("result"):
            tools = tools_response["result"].get("tools", [])
            print(f"✅ Found {len(tools)} tools:")
            for tool in tools:
                print(f"  - {tool['name']}: {tool['description']}")
        else:
            print("❌ Tools list failed")
            return False
        
        # Test 3: Call navigate tool
        print("\n🧭 Testing navigate tool...")
        navigate_response = send_request("tools/call", {
            "name": "navigate",
            "arguments": {
                "function": "process_data",
                "depth": 2
            }
        }, 3)
        
        if navigate_response and navigate_response.get("result"):
            print("✅ Navigate tool successful")
            content = navigate_response["result"].get("content", [])
            if content:
                print("📋 Result content:")
                for block in content:
                    if block.get("type") == "text":
                        result_data = json.loads(block["text"])
                        print(f"  Function: {result_data['function']['name']}")
                        print(f"  File: {result_data['function']['file']}:{result_data['function']['line']}")
                        print(f"  Calls: {len(result_data['calls'])} functions")
                        print(f"  Called by: {len(result_data['called_by'])} functions")
        else:
            print("❌ Navigate tool failed")
            return False
        
        # Test 4: Call find tool
        print("\n🔍 Testing find tool...")
        find_response = send_request("tools/call", {
            "name": "find",
            "arguments": {
                "query": "data"
            }
        }, 4)
        
        if find_response and find_response.get("result"):
            print("✅ Find tool successful")
            content = find_response["result"].get("content", [])
            if content:
                print("📋 Result content:")
                for block in content:
                    if block.get("type") == "text":
                        result_data = json.loads(block["text"])
                        print(f"  Found {len(result_data['matches'])} matches")
        else:
            print("❌ Find tool failed")
            return False
        
        print("\n🎉 All tests passed! MCP server is working correctly.")
        return True
        
    except Exception as e:
        print(f"❌ Test error: {e}")
        return False
    finally:
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    # First build the project
    print("🔨 Building project...")
    build_result = subprocess.run(['cargo', 'build'], capture_output=True, text=True)
    if build_result.returncode != 0:
        print(f"❌ Build failed: {build_result.stderr}")
        sys.exit(1)
    
    print("✅ Build successful")
    
    # Run the tests
    if test_mcp_server():
        print("\n✅ MCP server test completed successfully!")
        sys.exit(0)
    else:
        print("\n❌ MCP server test failed!")
        sys.exit(1)