use std::env;
use std::process::Command;

const ALLOWED_COMMANDS: &[&str] = &["ls", "pwd", "cat", "echo"];

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

    let output = Command::new(command).args(&args[2..]).output();

    match output {
        Ok(out) => {
            println!("{}", String::from_utf8_lossy(&out.stdout));
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

