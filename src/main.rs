use axum::{routing::post, Json, Router};
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;
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

async fn execute(Json(payload): Json<CommandRequest>) -> String {
    // 🔒 Command whitelist
    if !ALLOWED_COMMANDS.contains(&payload.command.as_str()) {
        return "Command not allowed".into();
    }

    // 🔒 Resolve command
    let resolved_command = match resolve_command(&payload.command) {
        Some(cmd) => cmd,
        None => return "Command resolution failed".into(),
    };

    // 🔒 Argument validation
    for arg in &payload.args {
        if arg.contains("..") {
            return "Blocked: directory traversal".into();
        }

        if arg.starts_with("/") {
            return "Blocked: absolute path".into();
        }

        if arg.starts_with("-") && !ALLOWED_FLAGS.contains(&arg.as_str()) {
            return format!("Flag not allowed: {}", arg);
        }
    }

    // 🔒 Sandbox directory
    let sandbox_dir = dirs::home_dir().unwrap().join("ai-lab/sandbox");

    if !sandbox_dir.exists() {
        return "Sandbox directory missing".into();
    }

    // 🔒 Execute command
    let mut child = match Command::new(resolved_command)
        .current_dir(&sandbox_dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .args(&payload.args)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return format!("Execution failed: {}", e),
    };

    // 🔒 Timeout
    let timeout = Duration::from_secs(3);

    match child.wait_timeout(timeout).unwrap() {
        Some(_) => {
            let output = child.wait_with_output().unwrap();

            format!(
                "STDOUT:\n{}\nSTDERR:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        }
        None => {
            child.kill().unwrap();
            "Process killed (timeout)".into()
        }
    }
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

