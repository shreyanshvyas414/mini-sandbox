#!/bin/bash
set -euo pipefail

# Require at least a command
if [ -z "${1:-}" ]; then
  echo "Usage: agent_exec.sh <command> [args...]"
  exit 1
fi

COMMAND="$1"
shift

# Build the args JSON array safely using jq — never via string interpolation.
# This prevents shell injection through crafted argument values.
ARGS_JSON="[]"
for arg in "$@"; do
  ARGS_JSON=$(jq -n --argjson arr "$ARGS_JSON" --arg val "$arg" '$arr + [$val]')
done

# Build the full payload via jq so all values are properly escaped.
PAYLOAD=$(jq -n \
  --arg cmd "$COMMAND" \
  --argjson args "$ARGS_JSON" \
  '{command: $cmd, args: $args}')

# Call the sandbox API
RESPONSE=$(curl -s -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD")

# Pretty-print if jq is available
if command -v jq >/dev/null 2>&1; then
  echo "$RESPONSE" | jq
else
  echo "$RESPONSE"
fi
