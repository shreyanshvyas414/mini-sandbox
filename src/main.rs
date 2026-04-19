use std::env;
use std::process::Command;
use std::time::Duration;

use wait_timeout::ChildExt;

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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mini_sandbox <command> [args]");
        return;
    }

    let command = &args[1];

    // 🔒 Command whitelist
    if !ALLOWED_COMMANDS.contains(&command.as_str()) {
        eprintln!("Command not allowed");
        return;
    }

    // 🔒 Resolve command to absolute path
    let resolved_command = match resolve_command(command) {
        Some(path) => path,
        None => {
            eprintln!("Command resolution failed");
            return;
        }
    };

    // 🔒 Argument validation
    for arg in &args[2..] {
        // Block directory traversal
        if arg.contains("..") {
            eprintln!("Blocked: directory traversal detected");
            return;
        }

        // Block absolute paths
        if arg.starts_with("/") {
            eprintln!("Blocked: absolute paths not allowed");
            return;
        }

        // Restrict flags
        if arg.starts_with("-") {
            if !ALLOWED_FLAGS.contains(&arg.as_str()) {
                eprintln!("Flag not allowed: {}", arg);
                return;
            }
        }
    }

    // 🔒 Sandbox directory
    let sandbox_dir = dirs::home_dir().unwrap().join("ai-lab/sandbox");

    if !sandbox_dir.exists() {
        eprintln!("Sandbox directory does not exist: {:?}", sandbox_dir);
        return;
    }

    println!("Executing safely: {:?}", args);

    // 🔒 Spawn process
    let mut child = match Command::new(resolved_command)
        .current_dir(&sandbox_dir)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .args(&args[2..])
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to start process: {}", e);
            return;
        }
    };

    // 🔒 Timeout protection
    let timeout = Duration::from_secs(3);

    match child.wait_timeout(timeout).unwrap() {
        Some(_status) => {
            let output = child.wait_with_output().unwrap();

            println!("{}", String::from_utf8_lossy(&output.stdout));
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        None => {
            child.kill().unwrap();
            println!("Process killed (timeout)");
        }
    }
}

