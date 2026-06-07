use std::collections::HashMap;
use std::fs;

use fraction::Fraction;

use crate::ast::ASTNode;
use crate::builtin::register_builtins;
use crate::environment::Env;
use crate::parsers::Parser as SagParser;
use crate::token::TokenKind;
use crate::tokenizer::tokenize;
use crate::value::Value;

const MAGIC: &str = "SAGC1";

#[derive(Debug, Clone)]
enum Instr {
    PushNum(Fraction),
    PushString(String),
    PushBool(bool),
    PushVoid,
    LoadVar(String),
    StoreVar {
        name: String,
        is_new: bool,
    },
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    Xor,
    Neg,
    Not,
    MakeList(usize),
    Pop,
    Call {
        name: String,
        argc: usize,
    },
    Jump(String),
    JumpIfFalse(String),
    Label(String),
    SetupLoop(String),
    ForIter {
        state: String,
        var: String,
        end: String,
    },
    Return,
    Halt,
}

#[derive(Debug, Clone)]
struct CompiledFunction {
    params: Vec<String>,
    code: Vec<Instr>,
}

#[derive(Debug, Clone)]
struct Program {
    entry: Vec<Instr>,
    functions: HashMap<String, CompiledFunction>,
}

struct CompileContext {
    functions: HashMap<String, CompiledFunction>,
    next_label: usize,
    loop_stack: Vec<LoopLabels>,
}

struct LoopLabels {
    continue_label: String,
    break_label: String,
}

impl CompileContext {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
            next_label: 0,
            loop_stack: Vec::new(),
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.next_label);
        self.next_label += 1;
        label
    }
}

pub fn compile_file(input_path: &str, output_path: Option<&str>) -> Result<String, String> {
    let source = fs::read_to_string(input_path).map_err(|e| e.to_string())?;
    let program = compile_source(&source)?;
    let output_path = output_path
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.sagc", input_path));
    fs::write(&output_path, serialize_program(&program)).map_err(|e| e.to_string())?;
    Ok(output_path)
}

pub fn run_compiled_file(path: &str) -> Result<(), String> {
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let program = parse_program(&source)?;
    let mut vm = Vm::new(program);
    vm.run()
}

fn compile_source(source: &str) -> Result<Program, String> {
    let source_owned = source.to_string();
    let tokens = tokenize(&source_owned);
    let mut env = Env::new();
    let builtins = register_builtins(&mut env);
    let mut parser = SagParser::new(tokens.to_vec(), builtins);
    let asts = parser
        .parse_lines()
        .map_err(|e| e.message_with_source(source))?;

    let mut ctx = CompileContext::new();
    let mut entry = compile_sequence(&asts, &mut ctx)?;
    entry.push(Instr::Halt);
    Ok(Program {
        entry,
        functions: ctx.functions,
    })
}

fn compile_sequence(nodes: &[ASTNode], ctx: &mut CompileContext) -> Result<Vec<Instr>, String> {
    if nodes.is_empty() {
        return Ok(vec![Instr::PushVoid]);
    }

    let mut code = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        code.extend(compile_node(node, ctx)?);
        if index + 1 != nodes.len() {
            code.push(Instr::Pop);
        }
    }
    Ok(code)
}

fn compile_node(node: &ASTNode, ctx: &mut CompileContext) -> Result<Vec<Instr>, String> {
    match node {
        ASTNode::Literal { value, .. } => compile_literal(value),
        ASTNode::Variable { name, .. } => Ok(vec![Instr::LoadVar(name.clone())]),
        ASTNode::Assign {
            name,
            value,
            is_new,
            ..
        } => {
            let mut code = compile_node(value, ctx)?;
            code.push(Instr::StoreVar {
                name: name.clone(),
                is_new: *is_new,
            });
            Ok(code)
        }
        ASTNode::BinaryOp {
            left, op, right, ..
        } => {
            let mut code = compile_node(left, ctx)?;
            code.extend(compile_node(right, ctx)?);
            code.push(match op {
                TokenKind::Plus => Instr::Add,
                TokenKind::Minus => Instr::Sub,
                TokenKind::Mul => Instr::Mul,
                TokenKind::Div => Instr::Div,
                TokenKind::Mod => Instr::Mod,
                TokenKind::Pow => Instr::Pow,
                TokenKind::And => Instr::And,
                TokenKind::Or => Instr::Or,
                TokenKind::Xor => Instr::Xor,
                _ => return Err(format!("unsupported binary operator in compiler: {:?}", op)),
            });
            Ok(code)
        }
        ASTNode::PrefixOp { op, expr, .. } => {
            let mut code = compile_node(expr, ctx)?;
            code.push(match op {
                TokenKind::Minus => Instr::Neg,
                TokenKind::Identifier(name) if name == "not" => Instr::Not,
                _ => return Err(format!("unsupported prefix operator in compiler: {:?}", op)),
            });
            Ok(code)
        }
        ASTNode::Eq { left, right, .. } => compile_compare(left, right, Instr::Eq, ctx),
        ASTNode::Gt { left, right, .. } => compile_compare(left, right, Instr::Gt, ctx),
        ASTNode::Gte { left, right, .. } => compile_compare(left, right, Instr::Gte, ctx),
        ASTNode::Lt { left, right, .. } => compile_compare(left, right, Instr::Lt, ctx),
        ASTNode::Lte { left, right, .. } => compile_compare(left, right, Instr::Lte, ctx),
        ASTNode::Block { nodes, .. } => compile_sequence(nodes, ctx),
        ASTNode::Return { expr, .. } => {
            let mut code = compile_node(expr, ctx)?;
            code.push(Instr::Return);
            Ok(code)
        }
        ASTNode::If {
            condition,
            then,
            else_,
            ..
        } => {
            let else_label = ctx.fresh_label("else");
            let end_label = ctx.fresh_label("ifend");
            let mut code = compile_node(condition, ctx)?;
            code.push(Instr::JumpIfFalse(else_label.clone()));
            code.extend(compile_node(then, ctx)?);
            code.push(Instr::Jump(end_label.clone()));
            code.push(Instr::Label(else_label));
            if let Some(else_node) = else_ {
                code.extend(compile_node(else_node, ctx)?);
            } else {
                code.push(Instr::PushVoid);
            }
            code.push(Instr::Label(end_label));
            Ok(code)
        }
        ASTNode::Function {
            name,
            arguments,
            body,
            ..
        } => {
            let mut params = Vec::new();
            for arg in arguments {
                match arg {
                    ASTNode::Variable { name, .. } => params.push(name.clone()),
                    _ => return Err(format!("unsupported function parameter: {:?}", arg)),
                }
            }
            let mut body_code = compile_node(body, ctx)?;
            body_code.push(Instr::Return);
            ctx.functions.insert(
                name.clone(),
                CompiledFunction {
                    params,
                    code: body_code,
                },
            );
            Ok(vec![Instr::PushVoid])
        }
        ASTNode::FunctionCall {
            name, arguments, ..
        } => {
            let args = match arguments.as_ref() {
                ASTNode::FunctionCallArgs { args, .. } => args,
                _ => return Err("illegal function arguments".into()),
            };
            let mut code = Vec::new();
            for arg in args {
                code.extend(compile_node(arg, ctx)?);
            }
            code.push(Instr::Call {
                name: name.clone(),
                argc: args.len(),
            });
            Ok(code)
        }
        ASTNode::For {
            variable,
            iterable,
            body,
            ..
        } => {
            let loop_state = ctx.fresh_label("loopstate");
            let loop_head = ctx.fresh_label("loophead");
            let loop_end = ctx.fresh_label("loopend");
            ctx.loop_stack.push(LoopLabels {
                continue_label: loop_head.clone(),
                break_label: loop_end.clone(),
            });

            let mut code = compile_node(iterable, ctx)?;
            code.push(Instr::SetupLoop(loop_state.clone()));
            code.push(Instr::Label(loop_head.clone()));
            code.push(Instr::ForIter {
                state: loop_state,
                var: variable.clone(),
                end: loop_end.clone(),
            });
            code.extend(compile_node(body, ctx)?);
            code.push(Instr::Pop);
            code.push(Instr::Jump(loop_head));
            code.push(Instr::Label(loop_end));
            code.push(Instr::PushVoid);
            ctx.loop_stack.pop();
            Ok(code)
        }
        ASTNode::Break { .. } => {
            let labels = ctx
                .loop_stack
                .last()
                .ok_or_else(|| "break used outside of loop".to_string())?;
            Ok(vec![Instr::Jump(labels.break_label.clone())])
        }
        ASTNode::Continue { .. } => {
            let labels = ctx
                .loop_stack
                .last()
                .ok_or_else(|| "continue used outside of loop".to_string())?;
            Ok(vec![Instr::Jump(labels.continue_label.clone())])
        }
        ASTNode::CommentBlock { .. } => Ok(vec![Instr::PushVoid]),
        _ => Err(format!("unsupported node in compiler: {:?}", node)),
    }
}

fn compile_compare(
    left: &ASTNode,
    right: &ASTNode,
    op: Instr,
    ctx: &mut CompileContext,
) -> Result<Vec<Instr>, String> {
    let mut code = compile_node(left, ctx)?;
    code.extend(compile_node(right, ctx)?);
    code.push(op);
    Ok(code)
}

fn compile_literal(value: &Value) -> Result<Vec<Instr>, String> {
    match value {
        Value::Number(n) => Ok(vec![Instr::PushNum(n.clone())]),
        Value::String(s) => Ok(vec![Instr::PushString(s.clone())]),
        Value::Bool(b) => Ok(vec![Instr::PushBool(*b)]),
        Value::Void => Ok(vec![Instr::PushVoid]),
        Value::List(values) => {
            let mut code = Vec::new();
            for value in values {
                code.extend(compile_literal(value)?);
            }
            code.push(Instr::MakeList(values.len()));
            Ok(code)
        }
        _ => Err(format!("unsupported literal in compiler: {:?}", value)),
    }
}

fn serialize_program(program: &Program) -> String {
    let mut out = String::new();
    out.push_str(MAGIC);
    out.push('\n');
    out.push_str("ENTRY\n");
    for instr in &program.entry {
        serialize_instr(instr, &mut out);
    }
    out.push_str("END\n");

    let mut function_names = program.functions.keys().cloned().collect::<Vec<_>>();
    function_names.sort();
    for function_name in function_names {
        let function = &program.functions[&function_name];
        out.push_str(&format!(
            "FUNC {} {}\n",
            function_name,
            function.params.join(" ")
        ));
        for instr in &function.code {
            serialize_instr(instr, &mut out);
        }
        out.push_str("END\n");
    }

    out
}

fn serialize_instr(instr: &Instr, out: &mut String) {
    match instr {
        Instr::PushNum(n) => out.push_str(&format!("PUSH_NUM {}\n", n)),
        Instr::PushString(s) => out.push_str(&format!("PUSH_STR {:?}\n", s)),
        Instr::PushBool(b) => out.push_str(&format!("PUSH_BOOL {}\n", b)),
        Instr::PushVoid => out.push_str("PUSH_VOID\n"),
        Instr::LoadVar(name) => out.push_str(&format!("LOAD {}\n", name)),
        Instr::StoreVar { name, is_new } => out.push_str(&format!(
            "STORE {} {}\n",
            if *is_new { "NEW" } else { "SET" },
            name
        )),
        Instr::Add => out.push_str("ADD\n"),
        Instr::Sub => out.push_str("SUB\n"),
        Instr::Mul => out.push_str("MUL\n"),
        Instr::Div => out.push_str("DIV\n"),
        Instr::Mod => out.push_str("MOD\n"),
        Instr::Pow => out.push_str("POW\n"),
        Instr::Eq => out.push_str("EQ\n"),
        Instr::Neq => out.push_str("NEQ\n"),
        Instr::Lt => out.push_str("LT\n"),
        Instr::Lte => out.push_str("LTE\n"),
        Instr::Gt => out.push_str("GT\n"),
        Instr::Gte => out.push_str("GTE\n"),
        Instr::And => out.push_str("AND\n"),
        Instr::Or => out.push_str("OR\n"),
        Instr::Xor => out.push_str("XOR\n"),
        Instr::Neg => out.push_str("NEG\n"),
        Instr::Not => out.push_str("NOT\n"),
        Instr::MakeList(len) => out.push_str(&format!("MAKE_LIST {}\n", len)),
        Instr::Pop => out.push_str("POP\n"),
        Instr::Call { name, argc } => out.push_str(&format!("CALL {} {}\n", name, argc)),
        Instr::Jump(label) => out.push_str(&format!("JUMP {}\n", label)),
        Instr::JumpIfFalse(label) => out.push_str(&format!("JUMP_IF_FALSE {}\n", label)),
        Instr::Label(label) => out.push_str(&format!("LABEL {}\n", label)),
        Instr::SetupLoop(state) => out.push_str(&format!("SETUP_LOOP {}\n", state)),
        Instr::ForIter { state, var, end } => {
            out.push_str(&format!("FOR_ITER {} {} {}\n", state, var, end))
        }
        Instr::Return => out.push_str("RETURN\n"),
        Instr::Halt => out.push_str("HALT\n"),
    }
}

fn parse_program(source: &str) -> Result<Program, String> {
    let mut lines = source.lines();
    if lines.next() != Some(MAGIC) {
        return Err("invalid compiled file magic".into());
    }

    let mut entry = Vec::new();
    let mut functions = HashMap::new();

    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if line == "ENTRY" {
            entry = parse_block(&mut lines)?;
        } else if let Some(rest) = line.strip_prefix("FUNC ") {
            let mut parts = rest.split_whitespace();
            let name = parts
                .next()
                .ok_or_else(|| "missing function name".to_string())?
                .to_string();
            let params = parts.map(|s| s.to_string()).collect::<Vec<_>>();
            let code = parse_block(&mut lines)?;
            functions.insert(name, CompiledFunction { params, code });
        } else {
            return Err(format!("invalid section header: {}", line));
        }
    }

    Ok(Program { entry, functions })
}

fn parse_block<'a, I>(lines: &mut I) -> Result<Vec<Instr>, String>
where
    I: Iterator<Item = &'a str>,
{
    let mut code = Vec::new();
    for line in lines {
        if line == "END" {
            return Ok(code);
        }
        if line.is_empty() {
            continue;
        }
        code.push(parse_instr(line)?);
    }
    Err("unterminated block".into())
}

fn parse_instr(line: &str) -> Result<Instr, String> {
    if let Some(rest) = line.strip_prefix("PUSH_NUM ") {
        return Ok(Instr::PushNum(
            rest.parse::<Fraction>().map_err(|e| e.to_string())?,
        ));
    }
    if let Some(rest) = line.strip_prefix("PUSH_STR ") {
        return Ok(Instr::PushString(unescape_string(rest)?));
    }
    if let Some(rest) = line.strip_prefix("PUSH_BOOL ") {
        return Ok(Instr::PushBool(match rest {
            "true" => true,
            "false" => false,
            _ => return Err(format!("invalid bool literal: {}", line)),
        }));
    }
    if line == "PUSH_VOID" {
        return Ok(Instr::PushVoid);
    }
    if let Some(rest) = line.strip_prefix("LOAD ") {
        return Ok(Instr::LoadVar(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("STORE ") {
        let (mode, name) = rest
            .split_once(' ')
            .ok_or_else(|| format!("invalid store instruction: {}", line))?;
        return Ok(Instr::StoreVar {
            name: name.to_string(),
            is_new: mode == "NEW",
        });
    }
    if let Some(rest) = line.strip_prefix("MAKE_LIST ") {
        return Ok(Instr::MakeList(
            rest.parse::<usize>().map_err(|e| e.to_string())?,
        ));
    }
    if let Some(rest) = line.strip_prefix("CALL ") {
        let mut parts = rest.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| "missing call name".to_string())?;
        let argc = parts
            .next()
            .ok_or_else(|| "missing call argc".to_string())?
            .parse::<usize>()
            .map_err(|e| e.to_string())?;
        return Ok(Instr::Call {
            name: name.to_string(),
            argc,
        });
    }
    if let Some(rest) = line.strip_prefix("JUMP_IF_FALSE ") {
        return Ok(Instr::JumpIfFalse(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("JUMP ") {
        return Ok(Instr::Jump(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("LABEL ") {
        return Ok(Instr::Label(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("SETUP_LOOP ") {
        return Ok(Instr::SetupLoop(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("FOR_ITER ") {
        let mut parts = rest.split_whitespace();
        let state = parts
            .next()
            .ok_or_else(|| "missing loop state".to_string())?;
        let var = parts.next().ok_or_else(|| "missing loop var".to_string())?;
        let end = parts.next().ok_or_else(|| "missing loop end".to_string())?;
        return Ok(Instr::ForIter {
            state: state.to_string(),
            var: var.to_string(),
            end: end.to_string(),
        });
    }

    match line {
        "ADD" => Ok(Instr::Add),
        "SUB" => Ok(Instr::Sub),
        "MUL" => Ok(Instr::Mul),
        "DIV" => Ok(Instr::Div),
        "MOD" => Ok(Instr::Mod),
        "POW" => Ok(Instr::Pow),
        "EQ" => Ok(Instr::Eq),
        "NEQ" => Ok(Instr::Neq),
        "LT" => Ok(Instr::Lt),
        "LTE" => Ok(Instr::Lte),
        "GT" => Ok(Instr::Gt),
        "GTE" => Ok(Instr::Gte),
        "AND" => Ok(Instr::And),
        "OR" => Ok(Instr::Or),
        "XOR" => Ok(Instr::Xor),
        "NEG" => Ok(Instr::Neg),
        "NOT" => Ok(Instr::Not),
        "POP" => Ok(Instr::Pop),
        "RETURN" => Ok(Instr::Return),
        "HALT" => Ok(Instr::Halt),
        _ => Err(format!("unknown instruction: {}", line)),
    }
}

fn unescape_string(input: &str) -> Result<String, String> {
    if !(input.starts_with('"') && input.ends_with('"')) {
        return Err(format!("invalid quoted string: {}", input));
    }
    let mut out = String::new();
    let mut chars = input[1..input.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let escaped = chars
            .next()
            .ok_or_else(|| "incomplete escape".to_string())?;
        out.push(match escaped {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            other => other,
        });
    }
    Ok(out)
}

struct Frame {
    scopes: Vec<HashMap<String, Value>>,
    loop_states: HashMap<String, (Vec<Value>, usize)>,
    write_globals: bool,
}

impl Frame {
    fn root() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            loop_states: HashMap::new(),
            write_globals: true,
        }
    }

    fn local() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            loop_states: HashMap::new(),
            write_globals: false,
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    fn put_new(&mut self, name: &str, value: Value) {
        self.scopes
            .last_mut()
            .expect("frame always has one scope")
            .insert(name.to_string(), value);
    }

    fn put_existing(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }
}

struct Vm {
    program: Program,
    globals: HashMap<String, Value>,
}

impl Vm {
    fn new(program: Program) -> Self {
        Self {
            program,
            globals: HashMap::new(),
        }
    }

    fn run(&mut self) -> Result<(), String> {
        let mut frame = Frame::root();
        let entry = self.program.entry.clone();
        self.execute(&entry, &mut frame)?;
        Ok(())
    }

    fn execute(&mut self, code: &[Instr], frame: &mut Frame) -> Result<Value, String> {
        let labels = collect_labels(code);
        let mut stack = Vec::<Value>::new();
        let mut ip = 0usize;

        while ip < code.len() {
            match &code[ip] {
                Instr::PushNum(n) => stack.push(Value::Number(n.clone())),
                Instr::PushString(s) => stack.push(Value::String(s.clone())),
                Instr::PushBool(b) => stack.push(Value::Bool(*b)),
                Instr::PushVoid => stack.push(Value::Void),
                Instr::LoadVar(name) => {
                    if let Some(value) = frame.get(name).or_else(|| self.globals.get(name).cloned())
                    {
                        stack.push(value);
                    } else {
                        return Err(format!("undefined variable: {}", name));
                    }
                }
                Instr::StoreVar { name, is_new } => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    if *is_new {
                        frame.put_new(name, value.clone());
                        if frame.write_globals {
                            self.globals.insert(name.clone(), value.clone());
                        }
                    } else if !frame.put_existing(name, value.clone()) {
                        if self.globals.contains_key(name) {
                            self.globals.insert(name.clone(), value.clone());
                        } else {
                            frame.put_new(name, value.clone());
                            if frame.write_globals {
                                self.globals.insert(name.clone(), value.clone());
                            }
                        }
                    } else if frame.write_globals {
                        self.globals.insert(name.clone(), value.clone());
                    }
                    stack.push(value);
                }
                Instr::Add => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "add")?);
                }
                Instr::Sub => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "sub")?);
                }
                Instr::Mul => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "mul")?);
                }
                Instr::Div => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "div")?);
                }
                Instr::Mod => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "mod")?);
                }
                Instr::Pow => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "pow")?);
                }
                Instr::Eq => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "eq")?);
                }
                Instr::Neq => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "neq")?);
                }
                Instr::Lt => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "lt")?);
                }
                Instr::Lte => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "lte")?);
                }
                Instr::Gt => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "gt")?);
                }
                Instr::Gte => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "gte")?);
                }
                Instr::And => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "and")?);
                }
                Instr::Or => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "or")?);
                }
                Instr::Xor => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "xor")?);
                }
                Instr::Neg => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match value {
                        Value::Number(n) => stack.push(Value::Number(-n)),
                        _ => return Err("NEG expects number".into()),
                    }
                }
                Instr::Not => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match value {
                        Value::Bool(v) => stack.push(Value::Bool(!v)),
                        _ => return Err("NOT expects bool".into()),
                    }
                }
                Instr::MakeList(len) => {
                    let mut items = Vec::with_capacity(*len);
                    for _ in 0..*len {
                        items.push(stack.pop().ok_or_else(|| "stack underflow".to_string())?);
                    }
                    items.reverse();
                    stack.push(Value::List(items));
                }
                Instr::Pop => {
                    let _ = stack.pop();
                }
                Instr::Call { name, argc } => {
                    let mut args = Vec::with_capacity(*argc);
                    for _ in 0..*argc {
                        args.push(stack.pop().ok_or_else(|| "stack underflow".to_string())?);
                    }
                    args.reverse();
                    stack.push(self.call(name, args)?);
                }
                Instr::Jump(label) => {
                    ip = *labels
                        .get(label)
                        .ok_or_else(|| format!("unknown label: {}", label))?;
                    continue;
                }
                Instr::JumpIfFalse(label) => {
                    let condition = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match condition {
                        Value::Bool(false) => {
                            ip = *labels
                                .get(label)
                                .ok_or_else(|| format!("unknown label: {}", label))?;
                            continue;
                        }
                        Value::Bool(true) => {}
                        _ => return Err("condition must be bool".into()),
                    }
                }
                Instr::Label(_) => {}
                Instr::SetupLoop(state) => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match value {
                        Value::List(values) => {
                            frame.loop_states.insert(state.clone(), (values, 0));
                        }
                        _ => return Err("for loop expects list".into()),
                    }
                }
                Instr::ForIter { state, var, end } => {
                    let next_value = match frame.loop_states.get_mut(state) {
                        Some((values, index)) => {
                            if *index >= values.len() {
                                None
                            } else {
                                let value = values[*index].clone();
                                *index += 1;
                                Some(value)
                            }
                        }
                        None => return Err(format!("missing loop state: {}", state)),
                    };
                    if let Some(value) = next_value {
                        frame.put_new(var, value);
                    } else {
                        ip = *labels
                            .get(end)
                            .ok_or_else(|| format!("unknown label: {}", end))?;
                        continue;
                    }
                }
                Instr::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Void));
                }
                Instr::Halt => {
                    return Ok(stack.pop().unwrap_or(Value::Void));
                }
            }
            ip += 1;
        }

        Ok(stack.pop().unwrap_or(Value::Void))
    }

    fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "print" => {
                let output = args
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{}", output);
                Ok(Value::Void)
            }
            "len" => {
                if args.len() != 1 {
                    return Err("len() takes exactly one argument".into());
                }
                match &args[0] {
                    Value::List(values) => Ok(Value::Number(Fraction::from(values.len()))),
                    Value::String(s) => Ok(Value::Number(Fraction::from(s.len()))),
                    _ => Err("len() requires list or string".into()),
                }
            }
            "range" => builtin_range(args),
            _ => {
                let function = self
                    .program
                    .functions
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("missing compiled function: {}", name))?;
                if function.params.len() != args.len() {
                    return Err(format!("argument length mismatch for {}", name));
                }
                let mut frame = Frame::local();
                for (param, arg) in function.params.iter().zip(args) {
                    frame.put_new(param, arg);
                }
                let code = function.code.clone();
                self.execute(&code, &mut frame)
            }
        }
    }
}

fn collect_labels(code: &[Instr]) -> HashMap<String, usize> {
    let mut labels = HashMap::new();
    for (index, instr) in code.iter().enumerate() {
        if let Instr::Label(label) = instr {
            labels.insert(label.clone(), index);
        }
    }
    labels
}

fn pop2(stack: &mut Vec<Value>) -> Result<(Value, Value), String> {
    let right = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
    let left = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
    Ok((left, right))
}

fn binary_op(left: Value, right: Value, op: &str) -> Result<Value, String> {
    match (left, right, op) {
        (Value::Number(l), Value::Number(r), "add") => Ok(Value::Number(l + r)),
        (Value::Number(l), Value::Number(r), "sub") => Ok(Value::Number(l - r)),
        (Value::Number(l), Value::Number(r), "mul") => Ok(Value::Number(l * r)),
        (Value::Number(l), Value::Number(r), "div") => Ok(Value::Number(l / r)),
        (Value::Number(l), Value::Number(r), "mod") => Ok(Value::Number(l % r)),
        (Value::Number(l), Value::Number(r), "pow") => {
            let raw_numer = l.numer().unwrap().wrapping_pow(*r.numer().unwrap() as u32);
            let raw_denom = l.denom().unwrap().wrapping_pow(*r.numer().unwrap() as u32);
            Ok(Value::Number((raw_numer, raw_denom).into()))
        }
        (Value::String(l), Value::String(r), "add") => Ok(Value::String(l + &r)),
        (Value::String(l), other, "add") => Ok(Value::String(l + &other.to_string())),
        (Value::Bool(l), Value::Bool(r), "and") => Ok(Value::Bool(l && r)),
        (Value::Bool(l), Value::Bool(r), "or") => Ok(Value::Bool(l || r)),
        (Value::Bool(l), Value::Bool(r), "xor") => Ok(Value::Bool(l ^ r)),
        (l, r, _) => Err(format!("unsupported operation: {:?} {} {:?}", l, op, r)),
    }
}

fn compare_op(left: Value, right: Value, op: &str) -> Result<Value, String> {
    let result = match op {
        "eq" => left == right,
        "neq" => left != right,
        "lt" => match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => l < r,
            _ => return Err("LT expects numbers".into()),
        },
        "lte" => match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => l <= r,
            _ => return Err("LTE expects numbers".into()),
        },
        "gt" => match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => l > r,
            _ => return Err("GT expects numbers".into()),
        },
        "gte" => match (&left, &right) {
            (Value::Number(l), Value::Number(r)) => l >= r,
            _ => return Err("GTE expects numbers".into()),
        },
        _ => return Err(format!("unsupported comparison: {}", op)),
    };
    Ok(Value::Bool(result))
}

fn builtin_range(args: Vec<Value>) -> Result<Value, String> {
    let (start, end, step) = match args.as_slice() {
        [Value::Number(end)] => (0, *end.numer().unwrap() as i64, 1),
        [Value::Number(start), Value::Number(end)] => (
            *start.numer().unwrap() as i64,
            *end.numer().unwrap() as i64,
            1,
        ),
        [Value::Number(start), Value::Number(end), Value::Number(step)] => (
            *start.numer().unwrap() as i64,
            *end.numer().unwrap() as i64,
            *step.numer().unwrap() as i64,
        ),
        _ => return Err("range() takes 1-3 numeric arguments".into()),
    };

    if step == 0 {
        return Err("range() step cannot be zero".into());
    }

    let mut values = Vec::new();
    let mut current = start;
    if step > 0 {
        while current < end {
            values.push(Value::Number(Fraction::from(current)));
            current += step;
        }
    } else {
        while current > end {
            values.push(Value::Number(Fraction::from(current)));
            current += step;
        }
    }

    Ok(Value::List(values))
}
