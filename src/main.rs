use std::io::{self, Write};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::Command;

#[derive(Clone)]
enum CommandType {
    Builtin(fn(&str) -> ()),
}

struct Shell {
    commands: HashMap<String, CommandType>,
}

impl Shell {
    fn new() -> Self {
        let mut commands = HashMap::new();
        
        commands.insert("echo".to_string(), CommandType::Builtin(|arg| {
            println!("{}", arg);
        }));
        
        commands.insert("exit".to_string(), CommandType::Builtin(|arg| {
            match arg {
                "0" => std::process::exit(0),
                _ => println!("{}: invalid argument", arg),
            }
        }));
        
        commands.insert("type".to_string(), CommandType::Builtin(|arg| {
            if arg.is_empty() {
                println!("type: not enough arguments");
                return;
            }
            match arg {
                "echo" | "exit" | "type" => println!("{} is a shellob builtin", arg),
                cmd => {
                    if let Some(path) = Shell::find_in_path(cmd) {
                        println!("{} is {}", cmd, path);
                    } else {
                        println!("{}: not found", cmd);
                    }
                }
            }
        }));

        Shell { commands }
    }

    fn find_in_path(command: &str) -> Option<String> {
        env::var("PATH").ok()?.split(':')
            .map(|dir| format!("{}/{}", dir, command))
            .find(|path| Path::new(path).is_file())
    }

    fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '\'' => {
                    // Single quotes: preserve everything literally
                    while let Some(c) = chars.next() {
                        if c == '\'' {
                            break;
                        }
                        current.push(c);
                    }
                }
                '"' => {
                    // Double quotes: handle escape sequences
                    while let Some(c) = chars.next() {
                        match c {
                            '"' => break,
                            '\\' => {
                                if let Some(next) = chars.next() {
                                    match next {
                                        '\\' | '$' | '"' | '\n' => current.push(next),
                                        _ => {
                                            current.push('\\');
                                            current.push(next);
                                        }
                                    }
                                }
                            }
                            _ => current.push(c),
                        }
                    }
                }
                '\\' => {
                    // Backslash: escape the next character
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                ' ' => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    fn handle_command(&self, input: &str) {
        let tokens = Shell::tokenize(input);
        if tokens.is_empty() {
            return;
        }

        let command = &tokens[0];
        let arguments = &tokens[1..];

        if let Some(cmd_type) = self.commands.get(command) {
            match cmd_type {
                CommandType::Builtin(func) => func(&arguments.join(" ")),
            }
        } else if let Some(path) = Shell::find_in_path(command) {
            // Execute the external command
            match Command::new(path).args(arguments).output() {
                Ok(output) => {
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                    eprint!("{}", String::from_utf8_lossy(&output.stderr));
                }
                Err(e) => eprintln!("Error executing command: {}", e),
            }
        } else {
            println!("{}: command not found", command);
        }
    }
}

fn main() {
    let shell = Shell::new();
    let stdin = io::stdin();
    
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        shell.handle_command(input.trim());
    }
}
