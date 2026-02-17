#!/bin/bash
# Verify RLC configuration

echo "Verifying RLC configuration..."

# Check required files
required_files=(
    ".rlc/config/gates.yaml"
    ".rlc/agents/communication.yaml"
)

missing=0
for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ Missing: $file"
        missing=$((missing + 1))
    else
        echo "✅ Found: $file"
    fi
done

if [ $missing -gt 0 ]; then
    echo "❌ Configuration incomplete: $missing files missing"
    exit 1
fi

echo "✅ Configuration verified"
