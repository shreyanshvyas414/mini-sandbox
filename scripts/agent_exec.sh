#!/bin/bash

COMMAND=$1
shift

# Build JSON args array properly
if [ $# -eq 0 ]; then
  ARGS="[]"
else
  ARGS=$(printf '"%s",' "$@" | sed 's/,$//')
  ARGS="[$ARGS]"
fi

# Call sandbox API
curl -s -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d "{\"command\":\"$COMMAND\",\"args\":$ARGS}"
