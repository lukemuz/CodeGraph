#!/bin/bash

echo "🧪 Testing New CodeGraph Features"
echo "=================================="

# Build the latest version
echo "1. Building CodeGraph..."
cargo build --release

# Index a test project
echo "2. Indexing test project..."
./target/release/codegraph index test_project --force

# Test basic functionality
echo "3. Testing basic operations..."

# Test that the index was created with symbols (not just functions)
echo "   - Checking index file exists..."
if [ -f "test_project/.codegraph/index.bin" ]; then
    echo "     ✅ Index file created"
else
    echo "     ❌ Index file not found"
    exit 1
fi

# Test MCP server operations by creating a simple test
echo "4. Creating MCP test client..."
cat > test_mcp_operations.py << 'EOF'
import json
import subprocess
import sys

def test_mcp_operation(tool_name, params):
    """Test an MCP operation by sending JSON-RPC request"""
    
    # Start the MCP server
    process = subprocess.Popen(
        ['./target/release/codegraph', 'serve', '--index', 'test_project/.codegraph/index.bin'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # Send initialize request
    init_request = {
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "client_info": {"name": "test-client", "version": "1.0.0"}
        },
        "id": 1
    }
    
    # Send tool request
    tool_request = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": params
        },
        "id": 2
    }
    
    try:
        # Send requests
        process.stdin.write(json.dumps(init_request) + "\n")
        process.stdin.write(json.dumps(tool_request) + "\n")
        process.stdin.close()
        
        # Get response
        stdout, stderr = process.communicate(timeout=10)
        
        # Parse responses
        lines = stdout.strip().split('\n')
        if len(lines) >= 2:
            tool_response = json.loads(lines[1])
            return tool_response
        else:
            print(f"Unexpected response format: {stdout}")
            return None
            
    except Exception as e:
        print(f"Error testing {tool_name}: {e}")
        return None
    finally:
        process.terminate()

# Test the find operation
print("Testing 'find' operation...")
result = test_mcp_operation("find", {"query": "main"})
if result and result.get("result"):
    content = json.loads(result["result"]["content"][0]["text"])
    print(f"✅ Found {len(content['matches'])} symbols matching 'main'")
    if content["matches"]:
        print(f"   First match: {content['matches'][0]['name']} at {content['matches'][0]['file']}:{content['matches'][0]['line']}")
else:
    print("❌ Find operation failed")

# Test the navigate operation  
print("\nTesting 'navigate' operation...")
result = test_mcp_operation("navigate", {"function": "main"})
if result and result.get("result"):
    content = json.loads(result["result"]["content"][0]["text"])
    print(f"✅ Navigate found function: {content['function']['name']}")
    print(f"   Calls {len(content['calls'])} functions, called by {len(content['called_by'])}")
else:
    print("❌ Navigate operation failed")

# Test the impact operation
print("\nTesting 'impact' operation...")
result = test_mcp_operation("impact", {"function": "main"})
if result and result.get("result"):
    content = json.loads(result["result"]["content"][0]["text"])
    print(f"✅ Impact analysis complete")
    print(f"   Risk level: {content['risk_level']}")
    print(f"   Direct callers: {len(content['direct_callers'])}")
else:
    print("❌ Impact operation failed")

print("\n🎉 MCP operations test complete!")
EOF

echo "5. Running MCP operations test..."
python3 test_mcp_operations.py

echo ""
echo "🎯 Testing Complete!"
echo "Check output above for any issues."