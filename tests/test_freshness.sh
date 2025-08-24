#!/bin/bash

echo "Testing CodeGraph Lazy Refresh Feature"
echo "======================================="
echo ""

# Build the project
echo "1. Building CodeGraph..."
cargo build
echo ""

# Index the test project
echo "2. Creating initial index..."
cd test_project
../target/debug/codegraph index . --force
echo ""

# Check the index shows 30 functions
echo "3. Initial index should have 30 functions"
echo ""

# Modify a file after indexing
echo "4. Adding a new function to main.py..."
cat >> main.py << 'EOF'

def another_new_function():
    """Added to test freshness checking."""
    return "Freshness test!"
EOF
echo "Added new function to main.py"
echo ""

# Sleep briefly to ensure file timestamp is newer
sleep 1

# Show that the file is newer than index
echo "5. File timestamps:"
echo -n "Index: "
ls -la .codegraph/index.bin | awk '{print $6, $7, $8}'
echo -n "main.py: "
ls -la main.py | awk '{print $6, $7, $8}'
echo ""

# Test the serve command with auto-refresh
echo "6. Starting server with auto-refresh (will run for 5 seconds)..."
echo "   The server should detect the stale index and rebuild it."
echo ""
timeout 5 ../target/debug/codegraph serve --auto-refresh --refresh-interval 2 2>&1 | grep -E "(stale|rebuild|fresh)" &

# Wait and show if index was updated
sleep 6
echo ""
echo "7. Checking if index was rebuilt:"
echo -n "New index time: "
ls -la .codegraph/index.bin | awk '{print $6, $7, $8}'

# Re-index to check function count
echo ""
echo "8. Re-indexing to verify new function was found..."
../target/debug/codegraph index . --force

echo ""
echo "Test complete!"