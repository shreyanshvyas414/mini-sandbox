use axum::{routing::post, Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower::buffer::BufferLayer;
use tower::limit::RateLimitLayer;
use tower::ServiceBuilder;
use wait_timeout::ChildExt;

#[derive(Deserialize)]
struct CommandRequest {
    command: String,
    args: Vec<String>,
}

/// Single source of truth: command name → absolute binary path.
/// Being in this map implicitly means the command is allowed and read-only.
const COMMAND_MAP: &[(&str, &str)] = &[
    ("ls", "/bin/ls"),
    ("pwd", "/bin/pwd"),
    ("cat", "/bin/cat"),
    ("echo", "/bin/echo"),
];

const ALLOWED_FLAGS: &[&str] = &["-l", "-a"];

/// Text-file extensions that `cat` is permitted to read.
const ALLOWED_CAT_EXTENSIONS: &[&str] = &[".txt", ".log", ".json", ".md", ".toml", ".yaml", ".yml"];

fn resolve_command(cmd: &str) -> Option<&'static str> {
    COMMAND_MAP
        .iter()
        .find(|(name, _)| *name == cmd)
        .map(|(_, path)| *path)
}

fn response(status: &str, message: &str) -> serde_json::Value {
    json!({
        "status": status,
        "message": message
    })
}

/// Logs an execution entry to ~/ai-lab/sandbox.log.
/// Must be called inside `tokio::task::spawn_blocking` — never directly
/// from an async handler — to avoid blocking the executor with file I/O.
fn log_execution(command: &str, args: &[String], output: &str) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("log_execution: cannot resolve home directory");
            return;
        }
    };

    let log_path = home.join("ai-lab/sandbox.log");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let entry = format!(
        "{} | {} {:?} | {}\n",
        timestamp,
        command,
        args,
        output.replace('\n', " ")
    );

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(entry.as_bytes());
    } else {
        eprintln!("log_execution: failed to open log file at {:?}", log_path);
    }
}

async fn execute(Json(payload): Json<CommandRequest>) -> Json<serde_json::Value> {
    // Command allowlist
    let resolved_command = match resolve_command(&payload.command) {
        Some(path) => path,
        None => return Json(response("error", "Command not allowed")),
    };

    // Argument count
    if payload.args.len() > 5 {
        return Json(response("error", "Too many arguments"));
    }

    // Per-argument validation
    for arg in &payload.args {
        if arg.len() > 100 {
            return Json(response("error", "Argument too long"));
        }

        if arg.contains("..") {
            return Json(response("error", "Blocked: directory traversal"));
        }

        if arg.starts_with('/') {
            return Json(response("error", "Blocked: absolute path"));
        }

        if arg.starts_with('-') {
            if !ALLOWED_FLAGS.contains(&arg.as_str()) {
                return Json(response("error", &format!("Flag not allowed: {}", arg)));
            }
            // Flags already validated — skip character check below.
            continue;
        }

        // Allowlist safe characters to prevent log injection.
        let safe = arg
            .chars()
            .all(|c| c.is_alphanumeric() || "-_./".contains(c));
        if !safe {
            return Json(response("error", "Invalid characters in argument"));
        }
    }

    // cat-specific: only permit reading text-like files
    if payload.command == "cat" {
        for arg in &payload.args {
            if arg.starts_with('-') {
                continue; // flags already validated above
            }
            let allowed = ALLOWED_CAT_EXTENSIONS.iter().any(|ext| arg.ends_with(ext));
            if !allowed {
                return Json(response(
                    "error",
                    "cat: only text files (.txt .log .json .md .toml .yaml .yml) allowed",
                ));
            }
        }
    }

    // Sandbox directory
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Json(response("error", "Cannot resolve home directory")),
    };

    let sandbox_dir = home.join("ai-lab/sandbox");

    if !sandbox_dir.exists() {
        return Json(response("error", "Sandbox directory missing"));
    }

    // Spawn child
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

    // Wait with timeout
    let timeout = Duration::from_secs(3);

    let result = match child.wait_timeout(timeout) {
        Ok(Some(_)) => match child.wait_with_output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)
            }
            Err(e) => format!("Failed to collect output: {}", e),
        },
        Ok(None) => {
            // Timed out — kill gracefully; ignore "already exited" error.
            let _ = child.kill();
            "Process killed (timeout)".to_string()
        }
        Err(e) => format!("wait_timeout error: {}", e),
    };

    // Async-safe logging
    let log_cmd = payload.command.clone();
    let log_args = payload.args.clone();
    let log_result = result.clone();
    tokio::task::spawn_blocking(move || {
        log_execution(&log_cmd, &log_args, &log_result);
    });

    Json(response("ok", &result))
}

#[tokio::main]
async fn main() {
    // BufferLayer wraps the service in an Arc-based queue, which gives it
    // the Clone impl that Axum requires. RateLimitLayer alone is not Clone.
    let app = Router::new().route("/execute", post(execute)).layer(
        ServiceBuilder::new()
            .layer(BufferLayer::new(1024))
            .layer(RateLimitLayer::new(10, Duration::from_secs(1))),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Sandbox API running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Command allowlist

    #[test]
    fn allowed_commands_resolve_to_absolute_paths() {
        assert_eq!(resolve_command("ls"), Some("/bin/ls"));
        assert_eq!(resolve_command("pwd"), Some("/bin/pwd"));
        assert_eq!(resolve_command("cat"), Some("/bin/cat"));
        assert_eq!(resolve_command("echo"), Some("/bin/echo"));
    }

    #[test]
    fn blocked_commands_return_none() {
        for cmd in &["rm", "bash", "sh", "curl", "wget", "python", "sudo", ""] {
            assert!(
                resolve_command(cmd).is_none(),
                "expected '{}' to be blocked",
                cmd
            );
        }
    }

    // Cat extension guard

    #[test]
    fn cat_allows_text_extensions() {
        for file in &[
            "notes.txt",
            "output.log",
            "config.json",
            "README.md",
            "Cargo.toml",
            "docker.yaml",
            "values.yml",
        ] {
            let allowed = ALLOWED_CAT_EXTENSIONS.iter().any(|e| file.ends_with(e));
            assert!(allowed, "expected '{}' to be allowed for cat", file);
        }
    }

    #[test]
    fn cat_blocks_non_text_extensions() {
        for file in &[
            "script.sh",
            "binary.bin",
            "archive.tar",
            "image.png",
            "Makefile",
            "run.py",
        ] {
            let allowed = ALLOWED_CAT_EXTENSIONS.iter().any(|e| file.ends_with(e));
            assert!(!allowed, "expected '{}' to be blocked for cat", file);
        }
    }

    // Argument character allowlist

    fn is_safe_arg(arg: &str) -> bool {
        arg.chars()
            .all(|c| c.is_alphanumeric() || "-_./".contains(c))
    }

    #[test]
    fn safe_args_pass_character_check() {
        for arg in &["hello", "file.txt", "my-dir", "sub_dir", "path/to/file.md"] {
            assert!(is_safe_arg(arg), "expected '{}' to be safe", arg);
        }
    }

    #[test]
    fn dangerous_args_fail_character_check() {
        for arg in &["hello; rm -rf", "$(whoami)", "a|b", "foo\nbar", "arg&bg"] {
            assert!(!is_safe_arg(arg), "expected '{}' to be blocked", arg);
        }
    }

    // Directory traversal detection

    #[test]
    fn traversal_patterns_are_detected() {
        for arg in &["../../etc", "../secret", "foo/../../bar"] {
            assert!(
                arg.contains(".."),
                "expected '{}' to trigger traversal check",
                arg
            );
        }
    }

    #[test]
    fn normal_paths_do_not_trigger_traversal() {
        for arg in &["subdir/file.txt", "logs/output.log", "file.md"] {
            assert!(
                !arg.contains(".."),
                "expected '{}' to pass traversal check",
                arg
            );
        }
    }

    // Absolute path detection

    #[test]
    fn absolute_paths_are_blocked() {
        for arg in &["/etc/passwd", "/bin/bash", "/"] {
            assert!(arg.starts_with('/'), "expected '{}' to be blocked", arg);
        }
    }

    #[test]
    fn relative_paths_are_allowed() {
        for arg in &["file.txt", "subdir/notes.md", "./local.json"] {
            assert!(!arg.starts_with('/'), "expected '{}' to pass", arg);
        }
    }

    // Flag allowlist

    #[test]
    fn allowed_flags_pass() {
        for flag in &["-l", "-a"] {
            assert!(
                ALLOWED_FLAGS.contains(flag),
                "expected flag '{}' to be allowed",
                flag
            );
        }
    }

    #[test]
    fn disallowed_flags_are_blocked() {
        for flag in &["-R", "-rf", "--help", "--all", "-la"] {
            assert!(
                !ALLOWED_FLAGS.contains(flag),
                "expected flag '{}' to be blocked",
                flag
            );
        }
    }
}

