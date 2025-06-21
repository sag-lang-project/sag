mod builtin;
mod environment;
mod tokenizer;
mod wasm;
mod evals;
mod parsers;
mod ast;
mod value;
mod token;
mod install;

use crate::builtin::register_builtins;
use crate::environment::Env;
use crate::evals::{eval, evals};
use crate::parsers::Parser as SagParser;
use crate::tokenizer::tokenize;
use crate::install::install_package;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        package_or_path: String
    },
    Run {
        file_path: String,
    },
    Repl,
}

fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = Env::new();
    let builtins = register_builtins(&mut env);
    for line in std::io::stdin().lines() {
        let line = line?;
        let tokens = tokenize(&line);
        let mut parser = SagParser::new(tokens.to_vec(), builtins.clone());
        let ast_node = parser.parse();
        if let Err(e) = ast_node {
            eprint!("{}", e.message_with_source(&line));
            continue;
        }
        println!("ast: {:?}", ast_node);
        let result = eval(ast_node.unwrap(), &mut env);
        println!("---------");
        match result {
            Ok(value) => println!("res: {:?}", value),
            Err(e) => eprint!("{}", e.message_with_source(&line)),
        }
    }
    Ok(())
}

fn run_file(file_path: String, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::read_to_string(file_path)?;

    let tokens = tokenize(&file);
    if debug {
        println!("tokens: {:?}", tokens);
    }
    let mut env = Env::new();
    let builtins = register_builtins(&mut env);
    let mut parser = SagParser::new(tokens.to_vec(), builtins.clone());
    let ast_nodes = parser.parse_lines();
    if let Err(e) = ast_nodes {
        eprint!("{}", e.message_with_source(&file));
        return Ok(());
    }
    if debug {
        println!("ast: {:?}", ast_nodes);
    }
    let result = evals(ast_nodes.unwrap(), &mut env);
    if let Err(e) = result {
        eprint!("{}", e.message_with_source(&file));
        return Ok(());
    }
    if debug {
        println!("env: {:?}", env);
    }
    Ok(())
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Install {package_or_path} => {
            install_package(package_or_path);
        }
        Commands::Run {file_path} => {
            let debug = false; // Set to true if you want debug mode
            if let Err(e) = run_file(file_path, debug) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Repl => {
            if let Err(e) = run_repl() {
                eprintln!("Error: {}", e);
            }
        }
    }
}
