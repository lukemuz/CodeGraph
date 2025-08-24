#!/usr/bin/env python3
"""
Simple test client for the MCP server
"""
import socket
import json
import uuid

def send_mcp_request(operation, host="127.0.0.1", port=3001):
    """Send a request to the MCP server."""
    try:
        # Create socket and connect
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((host, port))
        
        # Create request - operation should be embedded in request
        request = {
            "id": str(uuid.uuid4()),
            **operation  # Spread the operation fields into request
        }
        
        # Send request
        request_json = json.dumps(request) + "\n"
        sock.send(request_json.encode())
        
        # Receive response
        response = sock.recv(4096).decode().strip()
        sock.close()
        
        return json.loads(response)
    except Exception as e:
        return {"error": str(e)}

def test_navigate():
    """Test the Navigate operation."""
    print("🔍 Testing Navigate operation...")
    
    # Match the Rust enum structure
    operation = {
        "operation": "Navigate",
        "function": "hello_world",
        "depth": 2
    }
    
    response = send_mcp_request(operation)
    print(f"Response: {json.dumps(response, indent=2)}")
    return response

def test_find():
    """Test the Find operation."""
    print("\n🔍 Testing Find operation...")
    
    operation = {
        "operation": "Find",
        "query": "process",
        "scope": None
    }
    
    response = send_mcp_request(operation)
    print(f"Response: {json.dumps(response, indent=2)}")
    return response

def test_impact():
    """Test the Impact operation."""
    print("\n🔍 Testing Impact operation...")
    
    operation = {
        "operation": "Impact",
        "function": "clean_data",
        "include_tests": False
    }
    
    response = send_mcp_request(operation)
    print(f"Response: {json.dumps(response, indent=2)}")
    return response

if __name__ == "__main__":
    print("Testing CodeGraph MCP Server\n" + "="*40)
    
    # Test all operations
    test_navigate()
    test_find() 
    test_impact()
    
    print("\n✅ MCP testing complete!")