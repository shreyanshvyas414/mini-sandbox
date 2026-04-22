#!/bin/bash

#  Require at least a command
if [ -z "$1" ]; then
  echo "Usage: agent_exec.sh <command> [args]"
  exit 1
fi

COMMAND=$1
shift

#  Build JSON args safely
if [ $# -eq 0 ]; then
  ARGS="[]"
else
  ARGS=$(printf '"%s",' "$@" | sed 's/,$//')
  ARGS="[$ARGS]"
fi

#  Call API
RESPONSE=$(curl -s -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d "{\"command\":\"$COMMAND\",\"args\":$ARGS}")

#  Pretty print if jq is available
if command -v jq >/dev/null 2>&1; then
  echo "$RESPONSE" | jq
else
  echo "$RESPONSE"
fi
