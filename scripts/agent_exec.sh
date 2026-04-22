#!/bin/bash

if [ -z "$1" ]; then
  echo "Usage: agent_exec.sh <command> [args]"
  exit 1
fi

COMMAND=$1
shift

if [ $# -eq 0 ]; then
  ARGS="[]"
else
  ARGS=$(printf '"%s",' "$@" | sed 's/,$//')
  ARGS="[$ARGS]"
fi

curl -s -X POST http://localhost:3000/execute \
  -H "Content-Type: application/json" \
  -d "{\"command\":\"$COMMAND\",\"args\":$ARGS}"
