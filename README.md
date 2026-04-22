# Mini Sandbox

A minimal Rust execution layer designed to run commands for AI agents safely. It acts as a gatekeeper, ensuring that model-generated code stays within defined security boundaries.

## 🛠 How It Works

Instead of allowing an AI to access your shell directly, this project enforces a controlled pipeline:

LLM -> API -> Sandbox -> System

## ⚙️ Features

* Whitelisted Commands: Only ls, pwd, cat, and echo are permitted.
* Path Security: Blocks absolute paths and directory traversal (..).
* Filesystem Jail: All operations are pinned to ~/ai-lab/sandbox.
* Execution Guard: Automatic 3-second timeout to prevent hanging processes.
* Environment Isolation: Clears environment variables before execution.

## 🚀 Setup

1. Clone the repository:
```
git clone https://github.com/YOUR_USERNAME/mini-sandbox.git
cd mini-sandbox
```

2. Create the sandbox directory:
```
mkdir -p ~/ai-lab/sandbox
```

3. Launch the server:
```
cargo run
```

*The API will be running at http://127.0.0.1:3000*

## 💻 Usage

Via Script:
```
./scripts/agent_exec.sh ls
```

Via Curl:
```
curl -X POST http://localhost:3000/execute \
-H "Content-Type: application/json" \
-d '{"command":"ls","args":[]}'
```

## 🔒 Security & Limitations

* Target: Designed for local development and research.
* Auth: No built-in authentication (Localhost only).
* Isolation: Enforced via software logic rather than kernel-level containers.

---
**Philosophy:** Don't trust the model. Control the execution.
