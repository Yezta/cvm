#!/bin/zsh

# Source the updated .zshrc
source ~/.zshrc

# Test that python resolves to JCVM's version
echo "Testing JCVM Python integration..."
echo ""
echo "which python: $(which python)"
echo "python --version: $(python --version)"
echo ""
echo "Expected: Python 3.10.10 from $JCVM_DIR/alias/python/current/bin/python"
echo ""

# Check PATH order
echo "PATH entries (JCVM-related only):"
echo "$PATH" | tr ':' '\n' | grep -E "jcvm|pyenv" | nl
echo ""

# Show what's in the Python bin directory
echo "JCVM Python bin contents:"
ls -la $JCVM_DIR/alias/python/current/bin/ | grep python
