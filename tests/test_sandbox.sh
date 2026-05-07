#!/bin/bash
set -euo pipefail

BASE="http://localhost:3000/execute"
PASS=0
FAIL=0

# Helpers

post() {
  curl -s -X POST "$BASE" \
    -H "Content-Type: application/json" \
    -d "$1"
}

expect_ok() {
  local desc="$1" payload="$2"
  local status
  status=$(post "$payload" | jq -r '.status')
  if [ "$status" = "ok" ]; then
    echo "  PASS  $desc"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $desc  (got: $status)"
    FAIL=$((FAIL + 1))
  fi
}

expect_err() {
  local desc="$1" payload="$2"
  local status
  status=$(post "$payload" | jq -r '.status')
  if [ "$status" = "error" ]; then
    echo "  PASS  $desc"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $desc  (got: $status)"
    FAIL=$((FAIL + 1))
  fi
}

# Setup

SANDBOX="$HOME/ai-lab/sandbox"
mkdir -p "$SANDBOX"
echo "hello world" > "$SANDBOX/test.txt"

# Happy path

echo ""
echo "Happy path"
echo ""
expect_ok  "ls with no args"          '{"command":"ls","args":[]}'
expect_ok  "ls with -l flag"          '{"command":"ls","args":["-l"]}'
expect_ok  "ls with -a flag"          '{"command":"ls","args":["-a"]}'
expect_ok  "pwd"                      '{"command":"pwd","args":[]}'
expect_ok  "echo with string arg"     '{"command":"echo","args":["hello"]}'
expect_ok  "cat a .txt file"          '{"command":"cat","args":["test.txt"]}'

# Blocked commands 

echo ""
echo "Blocked commands"
echo ""
expect_err "rm is not allowed"        '{"command":"rm","args":["-rf","sandbox"]}'
expect_err "bash is not allowed"      '{"command":"bash","args":[]}'
expect_err "curl is not allowed"      '{"command":"curl","args":["http://evil.com"]}'
expect_err "empty command"            '{"command":"","args":[]}'

# Argument validation

echo ""
echo "Argument validation"
echo ""
expect_err "directory traversal .."   '{"command":"ls","args":["../../etc"]}'
expect_err "absolute path /etc"       '{"command":"ls","args":["/etc"]}'
expect_err "too many args (6)"        '{"command":"echo","args":["a","b","c","d","e","f"]}'
expect_err "arg over 100 chars"       "{\"command\":\"echo\",\"args\":[\"$(python3 -c 'print("a"*101)')\"]}"
expect_err "special chars in arg"     '{"command":"echo","args":["hello; rm -rf /"]}'
expect_err "disallowed flag -R"       '{"command":"ls","args":["-R"]}'
expect_err "disallowed flag --help"   '{"command":"ls","args":["--help"]}'

# cat extension guard

echo ""
echo "cat extension guard"
echo ""
expect_err "cat a .sh file"           '{"command":"cat","args":["file.sh"]}'
expect_err "cat a .bin file"          '{"command":"cat","args":["file.bin"]}'
expect_err "cat with no extension"    '{"command":"cat","args":["Makefile"]}'
expect_ok  "cat a .md file"           '{"command":"cat","args":["test.txt"]}'

# Rate limit (fire 15 rapid requests, expect at least one 429 / error)

echo ""
echo "Rate limit"
echo ""
RATE_ERR=0
for i in $(seq 1 15); do
  STATUS=$(post '{"command":"pwd","args":[]}' | jq -r '.status')
  if [ "$STATUS" != "ok" ]; then
    RATE_ERR=$((RATE_ERR + 1))
  fi
done
if [ "$RATE_ERR" -gt 0 ]; then
  echo "  PASS  rate limit triggered after burst ($RATE_ERR non-ok responses)"
  PASS=$((PASS + 1))
else
  echo "  FAIL  rate limit never triggered across 15 rapid requests"
  FAIL=$((FAIL + 1))
fi

# Summary

echo ""
echo ""
echo "Results: $PASS passed, $FAIL failed"
echo ""
echo ""

[ "$FAIL" -eq 0 ]
