import subprocess
import json

ALLOWED = ["ls", "pwd", "cat", "echo"]


def extract_json(text: str) -> str | None:
    """Strip markdown fences and model stop tokens, then extract the first JSON object."""
    text = text.replace("```json", "").replace("```", "")
    text = text.split("<|end|>")[0]

    start = text.find("{")
    end = text.rfind("}")

    if start != -1 and end != -1 and end > start:
        return text[start:end + 1]

    return None


def is_valid(data: dict) -> bool:
    return bool(data) and data.get("command") in ALLOWED


def run_model() -> str:
    """Invoke the local MLX model and return its raw text output."""
    return subprocess.check_output(
        [
            "python", "-m", "mlx_lm.generate",
            "--model", "mlx-community/Phi-3-mini-4k-instruct-4bit",
            "--max-tokens", "50",
            "--prompt",
            (
                "You are a JSON generator.\n"
                "Return ONLY a valid JSON object.\n"
                "No markdown. No explanation. No extra text.\n"
                'Format exactly: {"command":"ls","args":[]}\n'
                "Allowed commands: ls, pwd, cat, echo.\n"
                "Task: list files."
            ),
        ],
        timeout=30,  # prevent hanging if the model stalls
    ).decode()


# Retry loop — try up to 3 times to get a valid command from the model.
data = None
for attempt in range(1, 4):
    try:
        output = run_model()
    except subprocess.TimeoutExpired:
        print(f"Attempt {attempt}: model timed out, retrying...")
        continue
    except subprocess.CalledProcessError as exc:
        print(f"Attempt {attempt}: model process failed (exit {exc.returncode}), retrying...")
        continue

    print(f"Attempt {attempt} — raw model output:\n{output}")

    json_str = extract_json(output)
    if not json_str:
        print(f"Attempt {attempt}: no JSON object found in output.")
        continue

    try:
        parsed = json.loads(json_str)
    except json.JSONDecodeError as exc:
        print(f"Attempt {attempt}: JSON parse error — {exc}")
        continue

    if is_valid(parsed):
        data = parsed
        break

    print(f"Attempt {attempt}: command '{parsed.get('command')}' not in allowlist.")

if not data:
    print("Failed to get a valid command after 3 attempts.")
    raise SystemExit(1)

print("Parsed command:", data)

# Send to the sandbox via the shell script.
try:
    result = subprocess.check_output(
        ["./scripts/agent_exec.sh", data["command"], *data["args"]],
        timeout=10,  # guard against sandbox hanging
    ).decode()
except subprocess.TimeoutExpired:
    print("Sandbox request timed out.")
    raise SystemExit(1)
except subprocess.CalledProcessError as exc:
    print(f"agent_exec.sh failed with exit code {exc.returncode}.")
    raise SystemExit(1)

print("Sandbox response:\n", result)
