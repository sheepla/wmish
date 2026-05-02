use clap::{Parser, Subcommand};
use errors::AppError;

pub mod com;
pub mod completion;
pub mod errors;
pub mod parser;
pub mod shell;
pub mod wmi;

#[derive(Debug, Parser)]
#[clap(name = "wmish", version = "0.0.1", about = "WMI query tool")]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Interactive-shell mode
    Shell,
    /// Evaluate script and execute inline commands
    Run {
        /// Script file to run
        file: std::path::PathBuf,
    },
    /// Execute a single query
    Query {
        /// WQL query to execute
        query: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let _com = com::CoInitializer::new()?;

    match args.command {
        Command::Shell => {
            let mut shell = shell::Shell::new()?;
            shell.run()?;
        }
        Command::Run { file } => {
            let content = std::fs::read_to_string(file)?;
            let mut shell = shell::Shell::new()?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue; // Ignore empty lines and comments
                }
                match parser::parse_command(line) {
                    Ok((_, cmd)) => {
                        if let Err(e) = shell.execute_command(cmd) {
                            eprintln!("Error in script: {}", e);
                        }
                    }
                    Err(_) => {
                        if line.to_uppercase().starts_with("SELECT") {
                            if let Err(e) = shell.execute_query(line) {
                                eprintln!("Error in script query: {}", e);
                            }
                        } else {
                            eprintln!("Invalid command in script: {}", line);
                        }
                    }
                }
            }
        }
        Command::Query { query } => {
            let shell = shell::Shell::new()?;
            shell.execute_query(&query)?;
        }
    }

    Ok(())
}
