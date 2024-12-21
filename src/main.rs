use std::io::{self, Write};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use std::fs::File;

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
        
        commands.insert("cd".to_string(), CommandType::Builtin(|arg| {
            let new_dir = arg.split_whitespace().peekable().peek().map_or("/", |x| *x);
            let root = Path::new(new_dir);
            if let Err(e) = env::set_current_dir(&root) {
                eprintln!("{}", e);
            }
        }));

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
                "cd" | "echo" | "exit" | "type" => println!("{} is a shellob builtin", arg),
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

        // Find redirection operator and output file
        let mut cmd_end = tokens.len();
        let mut output_file = None;

        for i in 0..tokens.len() {
            if tokens[i] == ">" || tokens[i] == "1>" {
                if i + 1 < tokens.len() {
                    cmd_end = i;
                    output_file = Some(&tokens[i + 1]);
                }
                break;
            }
        }

        let command = &tokens[0];
        let arguments = &tokens[1..cmd_end];

        if let Some(cmd_type) = self.commands.get(command) {
            // Handle builtin commands
            match cmd_type {
                CommandType::Builtin(func) => {
                    if let Some(file) = output_file {
                        if let Ok(mut file) = File::create(file) {
                            let output = arguments.join(" ");
                            writeln!(file, "{}", output).unwrap_or_else(|e| eprintln!("Error writing to file: {}", e));
                        }
                    } else {
                        func(&arguments.join(" "))
                    }
                }
            }
        } else if let Some(path) = Shell::find_in_path(command) {
            // Execute the external command
            let path_clone = path.clone();
            let mut cmd = Command::new(path);
            cmd.args(arguments);

            if let Some(file) = output_file {
                if let Ok(file) = File::create(file) {
                    cmd.stdout(Stdio::from(file));
                } else {
                    eprintln!("Error: Could not create output file");
                    return;
                }
            }

            match cmd.output() {
                Ok(output) => {
                    if output_file.is_none() {
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                    }
                    let stderr = String::from_utf8_lossy(&output.stderr)
                        .replace(&format!("{}: ", path_clone), &format!("{}: ", command));
                    eprint!("{}", stderr);
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
