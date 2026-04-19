use std::env;
use std::process::Command;
use std::time::Duration;

use wait_timeout::ChildExt;

const ALLOWED_COMMANDS: &[&str] = &["ls", "pwd", "cat", "echo"];

fn resolve_command(cmd: &str) -> Option<&'static str> {
    match cmd {
        "ls" => Some("/bin/ls"),
        "pwd" => Some("/bin/pwd"),
        "cat" => Some("/bin/cat"),
        "echo" => Some("/bin/echo"),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mini_sandbox <command>");
        return;
    }

    let command = &args[1];

    if !ALLOWED_COMMANDS.contains(&command.as_str()) {
        eprintln!("Command not allowed");
        return;
    }

    let resolved = match resolve_command(command) {
        Some(c) => c,
        None => return,
    };

    let sandbox_dir = dirs::home_dir().unwrap().join("ai-lab/sandbox");

    let mut child = Command::new(resolved)
        .current_dir(&sandbox_dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .args(&args[2..])
        .spawn()
        .expect("failed");

    let timeout = Duration::from_secs(3);

    match child.wait_timeout(timeout).unwrap() {
        Some(_) => {
            let output = child.wait_with_output().unwrap();
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        None => {
            child.kill().unwrap();
            println!("Timeout");
        }
    }
}
