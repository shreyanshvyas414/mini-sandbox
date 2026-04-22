use axum::{routing::post, Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wait_timeout::ChildExt;

#[derive(Deserialize)]
struct CommandRequest {
    command: String,
    args: Vec<String>,
}

const ALLOWED_COMMANDS: &[&str] = &["ls", "pwd", "cat", "echo"];
const ALLOWED_FLAGS: &[&str] = &["-l", "-a"];

fn resolve_command(cmd: &str) -> Option<&'static str> {
    match cmd {
        "ls" => Some("/bin/ls"),
        "pwd" => Some("/bin/pwd"),
        "cat" => Some("/bin/cat"),
        "echo" => Some("/bin/echo"),
        _ => None,
    }
}

fn is_read_only_command(cmd: &str) -> bool {
    matches!(cmd, "ls" | "pwd" | "cat" | "echo")
}

fn response(status: &str, message: &str) -> serde_json::Value {
    json!({
        "status": status,
        "message": message
    })
}

fn log_execution(command: &str, args: &[String], output: &str) {
    let log_path = dirs::home_dir().unwrap().join("ai-lab/sandbox.log");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entry = format!(
        "{} | {} {:?} | {}\n",
        timestamp,
        command,
        args,
        output.replace("\n", " ")
    );

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(entry.as_bytes());
    }
}

async fn execute(Json(payload): Json<CommandRequest>) -> Json<serde_json::Value> {
    if !ALLOWED_COMMANDS.contains(&payload.command.as_str()) {
        return Json(response("error", "Command not allowed"));
    }

    let resolved_command = match resolve_command(&payload.command) {
        Some(cmd) => cmd,
        None => return Json(response("error", "Command resolution failed")),
    };

    if !is_read_only_command(&payload.command) {
        return Json(response("error", "Only read-only commands allowed"));
    }

    if payload.args.len() > 5 {
        return Json(response("error", "Too many arguments"));
    }

    for arg in &payload.args {
        if arg.contains("..") {
            return Json(response("error", "Blocked: directory traversal"));
        }

        if arg.starts_with("/") {
            return Json(response("error", "Blocked: absolute path"));
        }

        if arg.len() > 100 {
            return Json(response("error", "Argument too long"));
        }

        if arg.starts_with("-") && !ALLOWED_FLAGS.contains(&arg.as_str()) {
            return Json(response("error", &format!("Flag not allowed: {}", arg)));
        }
    }

    let sandbox_dir = dirs::home_dir().unwrap().join("ai-lab/sandbox");

    if !sandbox_dir.exists() {
        return Json(response("error", "Sandbox directory missing"));
    }

    // FIX: capture stdout & stderr
    let mut child = match Command::new(resolved_command)
        .current_dir(&sandbox_dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .args(&payload.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Json(response("error", &format!("Execution failed: {}", e))),
    };

    let timeout = Duration::from_secs(3);

    let result = match child.wait_timeout(timeout).unwrap() {
        Some(_) => {
            let output = child.wait_with_output().unwrap();

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);

            log_execution(&payload.command, &payload.args, &combined);

            combined
        }
        None => {
            child.kill().unwrap();
            let msg = "Process killed (timeout)".to_string();

            log_execution(&payload.command, &payload.args, &msg);

            msg
        }
    };

    Json(response("ok", &result))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/execute", post(execute));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Sandbox API running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

