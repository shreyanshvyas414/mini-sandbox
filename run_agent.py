import subprocess
import json

ALLOWED = ["ls", "pwd", "cat", "echo"]

def extract_json(text):
    # Remove markdown/code fences
    text = text.replace("```json", "").replace("```", "")

    # Cut off anything after model stop token
    text = text.split("<|end|>")[0]

    # Extract JSON safely
    start = text.find("{")
    end = text.rfind("}")

    if start != -1 and end != -1 and end > start:
        return text[start:end+1]

    return None

def is_valid(data):
    return data and data.get("command") in ALLOWED

def run_model():
    return subprocess.check_output([
        "python", "-m", "mlx_lm.generate",
        "--model", "mlx-community/Phi-3-mini-4k-instruct-4bit",
        "--max-tokens", "50",
        "--prompt",
        (
            "You are a JSON generator.\n"
            "Return ONLY a valid JSON object.\n"
            "No markdown. No explanation. No extra text.\n"
            "Format exactly: {\"command\":\"ls\",\"args\":[]}\n"
            "Allowed commands: ls, pwd, cat, echo.\n"
            "Task: list files."
        )
    ]).decode()

# 🔁 Retry loop
data = None
for _ in range(3):
    output = run_model()
    print("RAW MODEL OUTPUT:\n", output)

    json_str = extract_json(output)
    if not json_str:
        continue

    try:
        parsed = json.loads(json_str)
    except:
        continue

    if is_valid(parsed):
        data = parsed
        break

if not data:
    print("❌ Failed to get valid command")
    exit(1)

print("✅ Parsed Command:", data)

# 🚀 Send to sandbox
result = subprocess.check_output([
    "./scripts/agent_exec.sh",
    data["command"],
    *data["args"]
]).decode()

print("📦 Sandbox Response:\n", result)
