#!/usr/bin/env python3
"""
Test the function resolution by searching in our test project
"""

def test_function_search():
    """Test finding functions with different patterns."""
    import os
    import re
    
    test_dir = "test_project"
    
    # Test patterns (similar to what our Rust resolver does)
    patterns = [
        (r"def\s+hello_world\s*\(", "hello_world"),
        (r"def\s+process_data\s*\(", "process_data"), 
        (r"def\s+\w*process\w*\s*\(", "any process function"),
        (r"class\s+DataProcessor", "DataProcessor class"),
        (r"async\s+def\s+\w+", "async functions"),
    ]
    
    print("FUNCTION RESOLUTION TEST")
    print("="*40)
    
    for py_file in ["main.py", "utils.py"]:
        file_path = os.path.join(test_dir, py_file)
        if os.path.exists(file_path):
            print(f"\n📁 Searching in {py_file}:")
            
            with open(file_path, 'r') as f:
                content = f.read()
                lines = content.split('\n')
                
                for pattern, description in patterns:
                    regex = re.compile(pattern)
                    found = []
                    
                    for line_num, line in enumerate(lines, 1):
                        if regex.search(line):
                            found.append((line_num, line.strip()))
                    
                    if found:
                        print(f"  ✅ {description}:")
                        for line_num, line in found:
                            print(f"    Line {line_num}: {line[:60]}...")
                    else:
                        print(f"  ❌ {description}: Not found")

if __name__ == "__main__":
    test_function_search()