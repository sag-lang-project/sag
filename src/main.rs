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
mod rc_value;
mod rc_env;

use crate::builtin::register_builtins;
use crate::environment::Env;
use crate::rc_env::RcEnv;
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
        #[arg(short, long)]
        use_rc: bool,
    },
    Repl {
        #[arg(short, long)]
        use_rc: bool,
    },
}

fn run_repl_with_rc() -> Result<(), Box<dyn std::error::Error>> {
    // 初期環境を作成
    let mut env = Env::new();
    // ビルトイン関数を登録
    let builtins = register_builtins(&mut env);
    // Rc環境に変換
    let mut rc_env = RcEnv::from_env(&env);
    
    println!("SAG REPL (with Rc optimization)");
    println!("Type expressions to evaluate them. Press Ctrl+D to exit.");
    
    for line in std::io::stdin().lines() {
        let line = line?;
        let tokens = tokenize(&line);
        let mut parser = SagParser::new(tokens.to_vec(), builtins.clone());
        let ast_node = parser.parse();
        if let Err(e) = ast_node {
            eprint!("{}", e.message_with_source(&line));
            continue;
        }
        
        // RcEnvをEnvに変換して評価
        let mut temp_env = rc_env.to_env();
        let result = eval(ast_node.unwrap(), &mut temp_env);
        // 評価結果をRcEnvに戻す
        rc_env = RcEnv::from_env(&temp_env);
        
        match result {
            Ok(value) => println!("=> {}", value),
            Err(e) => eprint!("{}", e.message_with_source(&line)),
        }
    }
    Ok(())
}

fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = Env::new();
    let builtins = register_builtins(&mut env);
    
    println!("SAG REPL");
    println!("Type expressions to evaluate them. Press Ctrl+D to exit.");
    
    for line in std::io::stdin().lines() {
        let line = line?;
        let tokens = tokenize(&line);
        let mut parser = SagParser::new(tokens.to_vec(), builtins.clone());
        let ast_node = parser.parse();
        if let Err(e) = ast_node {
            eprint!("{}", e.message_with_source(&line));
            continue;
        }
        
        let result = eval(ast_node.unwrap(), &mut env);
        match result {
            Ok(value) => println!("=> {}", value),
            Err(e) => eprint!("{}", e.message_with_source(&line)),
        }
    }
    Ok(())
}

fn run_file_with_rc(file_path: String, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::read_to_string(file_path)?;

    let tokens = tokenize(&file);
    if debug {
        println!("tokens: {:?}", tokens);
    }
    
    // 初期環境を作成
    let mut env = Env::new();
    // ビルトイン関数を登録
    let builtins = register_builtins(&mut env);
    // Rc環境に変換
    let mut rc_env = RcEnv::from_env(&env);
    
    let mut parser = SagParser::new(tokens.to_vec(), builtins.clone());
    let ast_nodes = parser.parse_lines();
    if let Err(e) = ast_nodes {
        eprint!("{}", e.message_with_source(&file));
        return Ok(());
    }
    
    if debug {
        println!("ast: {:?}", ast_nodes);
    }
    
    // RcEnvをEnvに変換して評価
    let mut temp_env = rc_env.to_env();
    let result = evals(ast_nodes.unwrap(), &mut temp_env);
    // 評価結果をRcEnvに戻す
    rc_env = RcEnv::from_env(&temp_env);
    
    if let Err(e) = result {
        eprint!("{}", e.message_with_source(&file));
        return Ok(());
    }
    
    if debug {
        println!("env: {:?}", rc_env);
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
        Commands::Run {file_path, use_rc} => {
            let debug = false; // Set to true if you want debug mode
            
            if use_rc {
                println!("Running with Rc optimization");
                if let Err(e) = run_file_with_rc(file_path, debug) {
                    eprintln!("Error: {}", e);
                }
            } else {
                if let Err(e) = run_file(file_path, debug) {
                    eprintln!("Error: {}", e);
                }
            }
        }
        Commands::Repl {use_rc} => {
            if use_rc {
                println!("Starting REPL with Rc optimization");
                if let Err(e) = run_repl_with_rc() {
                    eprintln!("Error: {}", e);
                }
            } else {
                if let Err(e) = run_repl() {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }
}
