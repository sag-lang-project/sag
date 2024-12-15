mod tokenizer;
mod parser;
mod environment;
mod eval;

use std::env;
use crate::tokenizer::tokenize;
use crate::parser::Parser;
use crate::environment::Env;
use crate::eval::{eval, evals};


fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = Env::new();
    for line in std::io::stdin().lines() {
        let tokens = tokenize(&line?);
        println!("{:?}", tokens);
        let mut parser = Parser::new(tokens.to_vec());
        let ast_node = parser.parse();
        println!("{:?}", ast_node);
        let result = eval(ast_node, &mut env);
        println!("---------");
        println!("res: {:?}", result);
    }
    Ok(())
}

fn run_file(file_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::read_to_string(file_path)?;
    let tokens = tokenize(&file);
    println!("tokens: {:?}", tokens);
    let mut parser = Parser::new(tokens.to_vec());
    let ast_nodes = parser.parse_lines();
    println!("ast: {:?}", ast_nodes);
    let mut env = Env::new();
    let result = evals(ast_nodes, &mut env);
    println!("result: {:?}", result);
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        println!("args: {:?}", args);
        let file_path = args[1].clone();
        run_file(file_path).unwrap();
    } else {
        run_repl().unwrap();
    }
}


