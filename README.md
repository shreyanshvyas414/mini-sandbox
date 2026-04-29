# Mini Sandbox

A minimal Rust-based safety layer for executing AI-generated commands on local systems.

---

## Concept

AI → JSON → API → Sandbox → System

The AI suggests commands.  
Mini Sandbox validates and executes them safely.

---

## Architecture

![Architecture](architecture.svg)

---

## AI Integration

Mini Sandbox works with any local AI that can output JSON.

### Flow

AI → JSON → Mini Sandbox → Safe Execution

Example:

```
{ "command": "ls", "args": [] }
```

---

## MLX Integration (Local AI)

You can connect a local model using MLX.

Flow:

MLX → Python Bridge → Mini Sandbox → System

### Steps

1. Run the sandbox API  
   cargo run

2. Run the agent bridge  
   python examples/run_agent.py

This script:
- Generates a command using MLX
- Extracts valid JSON
- Sends it to the sandbox

> `run_agent.py` is a reference implementation (not required for usage)

---

## OpenClaw Integration

Flow:

OpenClaw → JSON → Mini Sandbox → Safe Execution

Use a bridge script to forward commands to the API.

---

## Features

- Whitelisted commands (ls, pwd, cat, echo)
- Blocks unsafe paths and arguments
- Runs inside ~/ai-lab/sandbox
- 3s execution timeout
- Clean environment (no leakage)
- Logs all executions

---

## Setup
```
git clone https://github.com/shreyanshvyas414/mini-sandbox.git
cd mini-sandbox

mkdir -p ~/ai-lab/sandbox

cargo run
```

---

## Usage

```
./scripts/agent_exec.sh ls
```

or

```
curl -X POST http://localhost:3000/execute \
-H "Content-Type: application/json" \
-d '{"command":"ls","args":[]}'
```
---

## Philosophy

Don't trust the model. Control the execution.
