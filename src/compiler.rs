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

const MAGIC_TEXT: &str = "SAGC1";
const MAGIC_BINARY: &[u8; 4] = b"SAGB";
const BINARY_VERSION_MAJOR: u16 = 1;
const BINARY_VERSION_MINOR: u16 = 0;
const BUILTIN_PRINT: u8 = 0;
const BUILTIN_LEN: u8 = 1;
const BUILTIN_RANGE: u8 = 2;

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

#[derive(Debug, Clone, PartialEq)]
enum BinaryConst {
    Number(Fraction),
    String(String),
    Bool(bool),
    Void,
}

#[derive(Debug, Clone)]
enum BinaryPseudoInstr {
    Const(u32),
    LoadGlobal(u32),
    StoreGlobal(u32),
    LoadLocal(u16),
    StoreLocal(u16),
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
    MakeList(u16),
    Pop,
    Call {
        target: BinaryCallTarget,
        argc: u16,
    },
    Jump(String),
    JumpIfFalse(String),
    Label(String),
    SetupLoop(u16),
    ForIter {
        loop_slot: u16,
        var_slot: u16,
        end_label: String,
    },
    Return,
    Halt,
}

#[derive(Debug, Clone)]
enum BinaryInstr {
    Const(u32),
    LoadGlobal(u32),
    StoreGlobal(u32),
    LoadLocal(u16),
    StoreLocal(u16),
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
    MakeList(u16),
    Pop,
    Call {
        target: BinaryCallTarget,
        argc: u16,
    },
    Jump(u32),
    JumpIfFalse(u32),
    SetupLoop(u16),
    ForIter {
        loop_slot: u16,
        var_slot: u16,
        end_ip: u32,
    },
    Return,
    Halt,
}

#[derive(Debug, Clone)]
struct BinaryFunction {
    arg_count: u16,
    local_count: u16,
    loop_count: u16,
    code: Vec<BinaryInstr>,
}

#[derive(Debug, Clone)]
struct BinaryProgram {
    constants: Vec<BinaryConst>,
    globals_count: u32,
    entry_local_count: u16,
    entry_loop_count: u16,
    entry: Vec<BinaryInstr>,
    functions: Vec<BinaryFunction>,
}

#[derive(Debug, Clone, Copy)]
enum BinaryCallTarget {
    Builtin(u8),
    Function(u16),
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
    let output = if output_path.ends_with(".sagb") {
        serialize_program_binary(&lower_program(&program)?)
    } else {
        serialize_program_text(&program).into_bytes()
    };
    fs::write(&output_path, output).map_err(|e| e.to_string())?;
    Ok(output_path)
}

pub fn run_compiled_file(path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let program = if bytes.starts_with(MAGIC_BINARY) {
        let program = parse_program_binary(&bytes)?;
        let mut vm = BinaryVm::new(program);
        return vm.run();
    } else {
        let source = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        parse_program_text(&source)?
    };
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

fn serialize_program_text(program: &Program) -> String {
    let mut out = String::new();
    out.push_str(MAGIC_TEXT);
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

fn serialize_program_binary(program: &BinaryProgram) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC_BINARY);
    write_u16(&mut out, BINARY_VERSION_MAJOR);
    write_u16(&mut out, BINARY_VERSION_MINOR);
    write_u32(&mut out, program.constants.len() as u32);
    for constant in &program.constants {
        write_constant(&mut out, constant);
    }
    write_u32(&mut out, program.globals_count);
    write_u16(&mut out, program.entry_local_count);
    write_u16(&mut out, program.entry_loop_count);
    write_binary_instr_block(&mut out, &program.entry);

    write_u32(&mut out, program.functions.len() as u32);
    for function in &program.functions {
        write_u16(&mut out, function.arg_count);
        write_u16(&mut out, function.local_count);
        write_u16(&mut out, function.loop_count);
        write_binary_instr_block(&mut out, &function.code);
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

fn parse_program_text(source: &str) -> Result<Program, String> {
    let mut lines = source.lines();
    if lines.next() != Some(MAGIC_TEXT) {
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

fn parse_program_binary(bytes: &[u8]) -> Result<BinaryProgram, String> {
    let mut reader = BinaryReader::new(bytes);
    reader.expect_magic(MAGIC_BINARY)?;
    let major = reader.read_u16()?;
    let minor = reader.read_u16()?;
    if major != BINARY_VERSION_MAJOR || minor != BINARY_VERSION_MINOR {
        return Err(format!("unsupported sagb version {}.{}", major, minor));
    }

    let constant_count = reader.read_u32()? as usize;
    let mut constants = Vec::with_capacity(constant_count);
    for _ in 0..constant_count {
        constants.push(read_constant(&mut reader)?);
    }
    let globals_count = reader.read_u32()?;
    let entry_local_count = reader.read_u16()?;
    let entry_loop_count = reader.read_u16()?;
    let entry = read_binary_instr_block(&mut reader)?;
    let function_count = reader.read_u32()? as usize;
    let mut functions = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        let arg_count = reader.read_u16()?;
        let local_count = reader.read_u16()?;
        let loop_count = reader.read_u16()?;
        let code = read_binary_instr_block(&mut reader)?;
        functions.push(BinaryFunction {
            arg_count,
            local_count,
            loop_count,
            code,
        });
    }

    if !reader.is_eof() {
        return Err("unexpected trailing bytes in sagb".into());
    }

    Ok(BinaryProgram {
        constants,
        globals_count,
        entry_local_count,
        entry_loop_count,
        entry,
        functions,
    })
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

struct BinaryLowering {
    constants: Vec<BinaryConst>,
    constant_indices: HashMap<String, u32>,
    globals: HashMap<String, u32>,
    function_indices: HashMap<String, u16>,
}

struct EntryLoweringContext {
    locals: HashMap<String, u16>,
    next_local: u16,
    next_loop: u16,
}

struct FunctionLoweringContext {
    locals: HashMap<String, u16>,
    next_local: u16,
    next_loop: u16,
}

fn lower_program(program: &Program) -> Result<BinaryProgram, String> {
    let mut lowering = BinaryLowering {
        constants: Vec::new(),
        constant_indices: HashMap::new(),
        globals: HashMap::new(),
        function_indices: HashMap::new(),
    };
    let mut function_names = program.functions.keys().cloned().collect::<Vec<_>>();
    function_names.sort();
    for (index, name) in function_names.iter().enumerate() {
        lowering.function_indices.insert(name.clone(), index as u16);
    }
    let mut entry_ctx = EntryLoweringContext {
        locals: HashMap::new(),
        next_local: 0,
        next_loop: 0,
    };
    let entry_pseudo = lower_entry_code(&program.entry, &mut lowering, &mut entry_ctx)?;
    let entry = resolve_binary_labels(entry_pseudo)?;

    let mut functions = Vec::with_capacity(function_names.len());
    for function_name in function_names {
        let function = &program.functions[&function_name];
        let mut ctx = FunctionLoweringContext {
            locals: HashMap::new(),
            next_local: 0,
            next_loop: 0,
        };
        for param in &function.params {
            ctx.locals.insert(param.clone(), ctx.next_local);
            ctx.next_local += 1;
        }
        let code = resolve_binary_labels(lower_function_code(
            &function.code,
            &mut lowering,
            &mut ctx,
        )?)?;
        functions.push(BinaryFunction {
            arg_count: function.params.len() as u16,
            local_count: ctx.next_local,
            loop_count: ctx.next_loop,
            code,
        });
    }

    Ok(BinaryProgram {
        constants: lowering.constants,
        globals_count: lowering.globals.len() as u32,
        entry_local_count: entry_ctx.next_local,
        entry_loop_count: entry_ctx.next_loop,
        entry,
        functions,
    })
}

fn lower_entry_code(
    code: &[Instr],
    lowering: &mut BinaryLowering,
    ctx: &mut EntryLoweringContext,
) -> Result<Vec<BinaryPseudoInstr>, String> {
    let mut out = Vec::with_capacity(code.len());
    for instr in code {
        lower_instr_for_entry(instr, lowering, ctx, &mut out)?;
    }
    Ok(out)
}

fn lower_function_code(
    code: &[Instr],
    lowering: &mut BinaryLowering,
    ctx: &mut FunctionLoweringContext,
) -> Result<Vec<BinaryPseudoInstr>, String> {
    let mut out = Vec::with_capacity(code.len());
    for instr in code {
        lower_instr_for_function(instr, lowering, ctx, &mut out)?;
    }
    Ok(out)
}

fn lower_instr_for_entry(
    instr: &Instr,
    lowering: &mut BinaryLowering,
    ctx: &mut EntryLoweringContext,
    out: &mut Vec<BinaryPseudoInstr>,
) -> Result<(), String> {
    match instr {
        Instr::PushNum(n) => out.push(BinaryPseudoInstr::Const(add_const(
            lowering,
            BinaryConst::Number(n.clone()),
        ))),
        Instr::PushString(s) => out.push(BinaryPseudoInstr::Const(add_const(
            lowering,
            BinaryConst::String(s.clone()),
        ))),
        Instr::PushBool(b) => out.push(BinaryPseudoInstr::Const(add_const(
            lowering,
            BinaryConst::Bool(*b),
        ))),
        Instr::PushVoid => out.push(BinaryPseudoInstr::Const(add_const(lowering, BinaryConst::Void))),
        Instr::LoadVar(name) => {
            if let Some(slot) = ctx.locals.get(name) {
                out.push(BinaryPseudoInstr::LoadLocal(*slot));
            } else if let Some(slot) = lowering.globals.get(name) {
                out.push(BinaryPseudoInstr::LoadGlobal(*slot));
            } else {
                return Err(format!(
                    "undefined global variable during binary lowering: {}",
                    name
                ));
            }
        }
        Instr::StoreVar { name, is_new } => {
            if *is_new {
                let slot = get_or_insert_global(&mut lowering.globals, name);
                out.push(BinaryPseudoInstr::StoreGlobal(slot));
            } else if let Some(slot) = ctx.locals.get(name) {
                out.push(BinaryPseudoInstr::StoreLocal(*slot));
            } else if let Some(slot) = lowering.globals.get(name) {
                out.push(BinaryPseudoInstr::StoreGlobal(*slot));
            } else {
                let slot = get_or_insert_global(&mut lowering.globals, name);
                out.push(BinaryPseudoInstr::StoreGlobal(slot));
            }
        }
        Instr::Add => out.push(BinaryPseudoInstr::Add),
        Instr::Sub => out.push(BinaryPseudoInstr::Sub),
        Instr::Mul => out.push(BinaryPseudoInstr::Mul),
        Instr::Div => out.push(BinaryPseudoInstr::Div),
        Instr::Mod => out.push(BinaryPseudoInstr::Mod),
        Instr::Pow => out.push(BinaryPseudoInstr::Pow),
        Instr::Eq => out.push(BinaryPseudoInstr::Eq),
        Instr::Neq => out.push(BinaryPseudoInstr::Neq),
        Instr::Lt => out.push(BinaryPseudoInstr::Lt),
        Instr::Lte => out.push(BinaryPseudoInstr::Lte),
        Instr::Gt => out.push(BinaryPseudoInstr::Gt),
        Instr::Gte => out.push(BinaryPseudoInstr::Gte),
        Instr::And => out.push(BinaryPseudoInstr::And),
        Instr::Or => out.push(BinaryPseudoInstr::Or),
        Instr::Xor => out.push(BinaryPseudoInstr::Xor),
        Instr::Neg => out.push(BinaryPseudoInstr::Neg),
        Instr::Not => out.push(BinaryPseudoInstr::Not),
        Instr::MakeList(len) => out.push(BinaryPseudoInstr::MakeList(*len as u16)),
        Instr::Pop => out.push(BinaryPseudoInstr::Pop),
        Instr::Call { name, argc } => out.push(BinaryPseudoInstr::Call {
            target: resolve_call_target(lowering, name)?,
            argc: *argc as u16,
        }),
        Instr::Jump(label) => out.push(BinaryPseudoInstr::Jump(label.clone())),
        Instr::JumpIfFalse(label) => out.push(BinaryPseudoInstr::JumpIfFalse(label.clone())),
        Instr::Label(label) => out.push(BinaryPseudoInstr::Label(label.clone())),
        Instr::SetupLoop(_) => {
            out.push(BinaryPseudoInstr::SetupLoop(ctx.next_loop));
            ctx.next_loop += 1;
        }
        Instr::ForIter { var, end, .. } => {
            let slot = get_or_insert_local(&mut ctx.locals, &mut ctx.next_local, var);
            let loop_slot = ctx
                .next_loop
                .checked_sub(1)
                .ok_or_else(|| "ForIter without prior SetupLoop in entry".to_string())?;
            out.push(BinaryPseudoInstr::ForIter {
                loop_slot,
                var_slot: slot,
                end_label: end.clone(),
            });
        }
        Instr::Return => out.push(BinaryPseudoInstr::Return),
        Instr::Halt => out.push(BinaryPseudoInstr::Halt),
    }
    Ok(())
}

fn lower_instr_for_function(
    instr: &Instr,
    lowering: &mut BinaryLowering,
    ctx: &mut FunctionLoweringContext,
    out: &mut Vec<BinaryPseudoInstr>,
) -> Result<(), String> {
    match instr {
        Instr::PushNum(n) => out.push(BinaryPseudoInstr::Const(add_const(
            lowering,
            BinaryConst::Number(n.clone()),
        ))),
        Instr::PushString(s) => out.push(BinaryPseudoInstr::Const(add_const(
            lowering,
            BinaryConst::String(s.clone()),
        ))),
        Instr::PushBool(b) => out.push(BinaryPseudoInstr::Const(add_const(
            lowering,
            BinaryConst::Bool(*b),
        ))),
        Instr::PushVoid => out.push(BinaryPseudoInstr::Const(add_const(lowering, BinaryConst::Void))),
        Instr::LoadVar(name) => {
            if let Some(slot) = ctx.locals.get(name) {
                out.push(BinaryPseudoInstr::LoadLocal(*slot));
            } else if let Some(slot) = lowering.globals.get(name) {
                out.push(BinaryPseudoInstr::LoadGlobal(*slot));
            } else {
                return Err(format!(
                    "undefined variable during function binary lowering: {}",
                    name
                ));
            }
        }
        Instr::StoreVar { name, is_new } => {
            if *is_new {
                let slot = get_or_insert_local(&mut ctx.locals, &mut ctx.next_local, name);
                out.push(BinaryPseudoInstr::StoreLocal(slot));
            } else if let Some(slot) = ctx.locals.get(name) {
                out.push(BinaryPseudoInstr::StoreLocal(*slot));
            } else if let Some(slot) = lowering.globals.get(name) {
                out.push(BinaryPseudoInstr::StoreGlobal(*slot));
            } else {
                let slot = get_or_insert_local(&mut ctx.locals, &mut ctx.next_local, name);
                out.push(BinaryPseudoInstr::StoreLocal(slot));
            }
        }
        Instr::Add => out.push(BinaryPseudoInstr::Add),
        Instr::Sub => out.push(BinaryPseudoInstr::Sub),
        Instr::Mul => out.push(BinaryPseudoInstr::Mul),
        Instr::Div => out.push(BinaryPseudoInstr::Div),
        Instr::Mod => out.push(BinaryPseudoInstr::Mod),
        Instr::Pow => out.push(BinaryPseudoInstr::Pow),
        Instr::Eq => out.push(BinaryPseudoInstr::Eq),
        Instr::Neq => out.push(BinaryPseudoInstr::Neq),
        Instr::Lt => out.push(BinaryPseudoInstr::Lt),
        Instr::Lte => out.push(BinaryPseudoInstr::Lte),
        Instr::Gt => out.push(BinaryPseudoInstr::Gt),
        Instr::Gte => out.push(BinaryPseudoInstr::Gte),
        Instr::And => out.push(BinaryPseudoInstr::And),
        Instr::Or => out.push(BinaryPseudoInstr::Or),
        Instr::Xor => out.push(BinaryPseudoInstr::Xor),
        Instr::Neg => out.push(BinaryPseudoInstr::Neg),
        Instr::Not => out.push(BinaryPseudoInstr::Not),
        Instr::MakeList(len) => out.push(BinaryPseudoInstr::MakeList(*len as u16)),
        Instr::Pop => out.push(BinaryPseudoInstr::Pop),
        Instr::Call { name, argc } => out.push(BinaryPseudoInstr::Call {
            target: resolve_call_target(lowering, name)?,
            argc: *argc as u16,
        }),
        Instr::Jump(label) => out.push(BinaryPseudoInstr::Jump(label.clone())),
        Instr::JumpIfFalse(label) => out.push(BinaryPseudoInstr::JumpIfFalse(label.clone())),
        Instr::Label(label) => out.push(BinaryPseudoInstr::Label(label.clone())),
        Instr::SetupLoop(_) => {
            out.push(BinaryPseudoInstr::SetupLoop(ctx.next_loop));
            ctx.next_loop += 1;
        }
        Instr::ForIter { var, end, .. } => {
            let slot = get_or_insert_local(&mut ctx.locals, &mut ctx.next_local, var);
            let loop_slot = ctx
                .next_loop
                .checked_sub(1)
                .ok_or_else(|| "ForIter without prior SetupLoop in function".to_string())?;
            out.push(BinaryPseudoInstr::ForIter {
                loop_slot,
                var_slot: slot,
                end_label: end.clone(),
            });
        }
        Instr::Return => out.push(BinaryPseudoInstr::Return),
        Instr::Halt => out.push(BinaryPseudoInstr::Halt),
    }
    Ok(())
}

fn add_const(lowering: &mut BinaryLowering, value: BinaryConst) -> u32 {
    let key = constant_key(&value);
    if let Some(index) = lowering.constant_indices.get(&key) {
        return *index;
    }

    let index = lowering.constants.len() as u32;
    lowering.constants.push(value);
    lowering.constant_indices.insert(key, index);
    index
}

fn constant_key(value: &BinaryConst) -> String {
    match value {
        BinaryConst::Number(number) => format!("n:{number}"),
        BinaryConst::String(text) => format!("s:{text:?}"),
        BinaryConst::Bool(boolean) => format!("b:{boolean}"),
        BinaryConst::Void => "v".to_string(),
    }
}

fn resolve_call_target(lowering: &BinaryLowering, name: &str) -> Result<BinaryCallTarget, String> {
    match name {
        "print" => Ok(BinaryCallTarget::Builtin(BUILTIN_PRINT)),
        "len" => Ok(BinaryCallTarget::Builtin(BUILTIN_LEN)),
        "range" => Ok(BinaryCallTarget::Builtin(BUILTIN_RANGE)),
        _ => lowering
            .function_indices
            .get(name)
            .copied()
            .map(BinaryCallTarget::Function)
            .ok_or_else(|| format!("unknown function during binary lowering: {}", name)),
    }
}

fn get_or_insert_global(globals: &mut HashMap<String, u32>, name: &str) -> u32 {
    if let Some(slot) = globals.get(name) {
        *slot
    } else {
        let slot = globals.len() as u32;
        globals.insert(name.to_string(), slot);
        slot
    }
}

fn get_or_insert_local(locals: &mut HashMap<String, u16>, next_local: &mut u16, name: &str) -> u16 {
    if let Some(slot) = locals.get(name) {
        *slot
    } else {
        let slot = *next_local;
        *next_local += 1;
        locals.insert(name.to_string(), slot);
        slot
    }
}

fn resolve_binary_labels(code: Vec<BinaryPseudoInstr>) -> Result<Vec<BinaryInstr>, String> {
    let mut labels = HashMap::new();
    let mut ip = 0u32;
    for instr in &code {
        match instr {
            BinaryPseudoInstr::Label(label) => {
                labels.insert(label.clone(), ip);
            }
            _ => ip += 1,
        }
    }

    let mut lowered = Vec::with_capacity(ip as usize);
    for instr in code {
        match instr {
            BinaryPseudoInstr::Const(index) => lowered.push(BinaryInstr::Const(index)),
            BinaryPseudoInstr::LoadGlobal(slot) => lowered.push(BinaryInstr::LoadGlobal(slot)),
            BinaryPseudoInstr::StoreGlobal(slot) => lowered.push(BinaryInstr::StoreGlobal(slot)),
            BinaryPseudoInstr::LoadLocal(slot) => lowered.push(BinaryInstr::LoadLocal(slot)),
            BinaryPseudoInstr::StoreLocal(slot) => lowered.push(BinaryInstr::StoreLocal(slot)),
            BinaryPseudoInstr::Add => lowered.push(BinaryInstr::Add),
            BinaryPseudoInstr::Sub => lowered.push(BinaryInstr::Sub),
            BinaryPseudoInstr::Mul => lowered.push(BinaryInstr::Mul),
            BinaryPseudoInstr::Div => lowered.push(BinaryInstr::Div),
            BinaryPseudoInstr::Mod => lowered.push(BinaryInstr::Mod),
            BinaryPseudoInstr::Pow => lowered.push(BinaryInstr::Pow),
            BinaryPseudoInstr::Eq => lowered.push(BinaryInstr::Eq),
            BinaryPseudoInstr::Neq => lowered.push(BinaryInstr::Neq),
            BinaryPseudoInstr::Lt => lowered.push(BinaryInstr::Lt),
            BinaryPseudoInstr::Lte => lowered.push(BinaryInstr::Lte),
            BinaryPseudoInstr::Gt => lowered.push(BinaryInstr::Gt),
            BinaryPseudoInstr::Gte => lowered.push(BinaryInstr::Gte),
            BinaryPseudoInstr::And => lowered.push(BinaryInstr::And),
            BinaryPseudoInstr::Or => lowered.push(BinaryInstr::Or),
            BinaryPseudoInstr::Xor => lowered.push(BinaryInstr::Xor),
            BinaryPseudoInstr::Neg => lowered.push(BinaryInstr::Neg),
            BinaryPseudoInstr::Not => lowered.push(BinaryInstr::Not),
            BinaryPseudoInstr::MakeList(len) => lowered.push(BinaryInstr::MakeList(len)),
            BinaryPseudoInstr::Pop => lowered.push(BinaryInstr::Pop),
            BinaryPseudoInstr::Call { target, argc } => {
                lowered.push(BinaryInstr::Call { target, argc })
            }
            BinaryPseudoInstr::Jump(label) => lowered.push(BinaryInstr::Jump(
                *labels
                    .get(&label)
                    .ok_or_else(|| format!("unknown binary label: {}", label))?,
            )),
            BinaryPseudoInstr::JumpIfFalse(label) => lowered.push(BinaryInstr::JumpIfFalse(
                *labels
                    .get(&label)
                    .ok_or_else(|| format!("unknown binary label: {}", label))?,
            )),
            BinaryPseudoInstr::Label(_) => {}
            BinaryPseudoInstr::SetupLoop(slot) => lowered.push(BinaryInstr::SetupLoop(slot)),
            BinaryPseudoInstr::ForIter {
                loop_slot,
                var_slot,
                end_label,
            } => lowered.push(BinaryInstr::ForIter {
                loop_slot,
                var_slot,
                end_ip: *labels
                    .get(&end_label)
                    .ok_or_else(|| format!("unknown binary loop label: {}", end_label))?,
            }),
            BinaryPseudoInstr::Return => lowered.push(BinaryInstr::Return),
            BinaryPseudoInstr::Halt => lowered.push(BinaryInstr::Halt),
        }
    }
    Ok(lowered)
}

fn write_constant(out: &mut Vec<u8>, constant: &BinaryConst) {
    match constant {
        BinaryConst::Number(value) => {
            out.push(0x01);
            write_string(out, &value.to_string());
        }
        BinaryConst::String(value) => {
            out.push(0x02);
            write_string(out, value);
        }
        BinaryConst::Bool(value) => {
            out.push(0x03);
            out.push(u8::from(*value));
        }
        BinaryConst::Void => out.push(0x04),
    }
}

fn read_constant(reader: &mut BinaryReader<'_>) -> Result<BinaryConst, String> {
    match reader.read_u8()? {
        0x01 => Ok(BinaryConst::Number(
            reader
                .read_string()?
                .parse::<Fraction>()
                .map_err(|e| e.to_string())?,
        )),
        0x02 => Ok(BinaryConst::String(reader.read_string()?)),
        0x03 => Ok(BinaryConst::Bool(reader.read_u8()? != 0)),
        0x04 => Ok(BinaryConst::Void),
        other => Err(format!("unknown sagb constant tag: 0x{:02x}", other)),
    }
}

fn write_binary_instr_block(out: &mut Vec<u8>, code: &[BinaryInstr]) {
    write_u32(out, code.len() as u32);
    for instr in code {
        write_binary_instr(out, instr);
    }
}

fn read_binary_instr_block(reader: &mut BinaryReader<'_>) -> Result<Vec<BinaryInstr>, String> {
    let len = reader.read_u32()? as usize;
    let mut code = Vec::with_capacity(len);
    for _ in 0..len {
        code.push(read_binary_instr(reader)?);
    }
    Ok(code)
}

fn write_binary_instr(out: &mut Vec<u8>, instr: &BinaryInstr) {
    match instr {
        BinaryInstr::Const(index) => {
            out.push(0x01);
            write_u32(out, *index);
        }
        BinaryInstr::LoadGlobal(slot) => {
            out.push(0x02);
            write_u32(out, *slot);
        }
        BinaryInstr::StoreGlobal(slot) => {
            out.push(0x03);
            write_u32(out, *slot);
        }
        BinaryInstr::LoadLocal(slot) => {
            out.push(0x04);
            write_u16(out, *slot);
        }
        BinaryInstr::StoreLocal(slot) => {
            out.push(0x05);
            write_u16(out, *slot);
        }
        BinaryInstr::Add => out.push(0x06),
        BinaryInstr::Sub => out.push(0x07),
        BinaryInstr::Mul => out.push(0x08),
        BinaryInstr::Div => out.push(0x09),
        BinaryInstr::Mod => out.push(0x0A),
        BinaryInstr::Pow => out.push(0x0B),
        BinaryInstr::Eq => out.push(0x0C),
        BinaryInstr::Neq => out.push(0x0D),
        BinaryInstr::Lt => out.push(0x0E),
        BinaryInstr::Lte => out.push(0x0F),
        BinaryInstr::Gt => out.push(0x10),
        BinaryInstr::Gte => out.push(0x11),
        BinaryInstr::And => out.push(0x12),
        BinaryInstr::Or => out.push(0x13),
        BinaryInstr::Xor => out.push(0x14),
        BinaryInstr::Neg => out.push(0x15),
        BinaryInstr::Not => out.push(0x16),
        BinaryInstr::MakeList(len) => {
            out.push(0x17);
            write_u16(out, *len);
        }
        BinaryInstr::Pop => out.push(0x18),
        BinaryInstr::Call { target, argc } => {
            out.push(0x19);
            match target {
                BinaryCallTarget::Builtin(index) => {
                    out.push(0);
                    out.push(*index);
                }
                BinaryCallTarget::Function(index) => {
                    out.push(1);
                    write_u16(out, *index);
                }
            }
            write_u16(out, *argc);
        }
        BinaryInstr::Jump(ip) => {
            out.push(0x1A);
            write_u32(out, *ip);
        }
        BinaryInstr::JumpIfFalse(ip) => {
            out.push(0x1B);
            write_u32(out, *ip);
        }
        BinaryInstr::SetupLoop(slot) => {
            out.push(0x1C);
            write_u16(out, *slot);
        }
        BinaryInstr::ForIter {
            loop_slot,
            var_slot,
            end_ip,
        } => {
            out.push(0x1D);
            write_u16(out, *loop_slot);
            write_u16(out, *var_slot);
            write_u32(out, *end_ip);
        }
        BinaryInstr::Return => out.push(0x1E),
        BinaryInstr::Halt => out.push(0x1F),
    }
}

fn read_binary_instr(reader: &mut BinaryReader<'_>) -> Result<BinaryInstr, String> {
    let opcode = reader.read_u8()?;
    match opcode {
        0x01 => Ok(BinaryInstr::Const(reader.read_u32()?)),
        0x02 => Ok(BinaryInstr::LoadGlobal(reader.read_u32()?)),
        0x03 => Ok(BinaryInstr::StoreGlobal(reader.read_u32()?)),
        0x04 => Ok(BinaryInstr::LoadLocal(reader.read_u16()?)),
        0x05 => Ok(BinaryInstr::StoreLocal(reader.read_u16()?)),
        0x06 => Ok(BinaryInstr::Add),
        0x07 => Ok(BinaryInstr::Sub),
        0x08 => Ok(BinaryInstr::Mul),
        0x09 => Ok(BinaryInstr::Div),
        0x0A => Ok(BinaryInstr::Mod),
        0x0B => Ok(BinaryInstr::Pow),
        0x0C => Ok(BinaryInstr::Eq),
        0x0D => Ok(BinaryInstr::Neq),
        0x0E => Ok(BinaryInstr::Lt),
        0x0F => Ok(BinaryInstr::Lte),
        0x10 => Ok(BinaryInstr::Gt),
        0x11 => Ok(BinaryInstr::Gte),
        0x12 => Ok(BinaryInstr::And),
        0x13 => Ok(BinaryInstr::Or),
        0x14 => Ok(BinaryInstr::Xor),
        0x15 => Ok(BinaryInstr::Neg),
        0x16 => Ok(BinaryInstr::Not),
        0x17 => Ok(BinaryInstr::MakeList(reader.read_u16()?)),
        0x18 => Ok(BinaryInstr::Pop),
        0x19 => {
            let kind = reader.read_u8()?;
            let target = match kind {
                0 => BinaryCallTarget::Builtin(reader.read_u8()?),
                1 => BinaryCallTarget::Function(reader.read_u16()?),
                _ => return Err(format!("unknown call target kind: {}", kind)),
            };
            Ok(BinaryInstr::Call {
                target,
                argc: reader.read_u16()?,
            })
        }
        0x1A => Ok(BinaryInstr::Jump(reader.read_u32()?)),
        0x1B => Ok(BinaryInstr::JumpIfFalse(reader.read_u32()?)),
        0x1C => Ok(BinaryInstr::SetupLoop(reader.read_u16()?)),
        0x1D => Ok(BinaryInstr::ForIter {
            loop_slot: reader.read_u16()?,
            var_slot: reader.read_u16()?,
            end_ip: reader.read_u32()?,
        }),
        0x1E => Ok(BinaryInstr::Return),
        0x1F => Ok(BinaryInstr::Halt),
        _ => Err(format!("unknown sagb opcode: 0x{:02x}", opcode)),
    }
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn expect_magic(&mut self, magic: &[u8]) -> Result<(), String> {
        let actual = self.read_exact(magic.len())?;
        if actual == magic {
            Ok(())
        } else {
            Err("invalid binary magic".into())
        }
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "binary offset overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("unexpected end of sagb".into());
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn is_eof(&self) -> bool {
        self.offset == self.bytes.len()
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

struct BinaryFrame {
    locals: Vec<Value>,
    loop_states: Vec<Option<(Vec<Value>, usize)>>,
}

struct BinaryVm {
    program: BinaryProgram,
    globals: Vec<Value>,
}

impl BinaryVm {
    fn new(program: BinaryProgram) -> Self {
        let globals = vec![Value::Void; program.globals_count as usize];
        Self { program, globals }
    }

    fn run(&mut self) -> Result<(), String> {
        let mut frame = BinaryFrame {
            locals: vec![Value::Void; self.program.entry_local_count as usize],
            loop_states: vec![None; self.program.entry_loop_count as usize],
        };
        let entry = self.program.entry.clone();
        let _ = self.execute(&entry, &mut frame)?;
        Ok(())
    }

    fn execute(&mut self, code: &[BinaryInstr], frame: &mut BinaryFrame) -> Result<Value, String> {
        let mut stack = Vec::<Value>::new();
        let mut ip = 0usize;

        while ip < code.len() {
            match &code[ip] {
                BinaryInstr::Const(index) => {
                    let value = self.constant_to_value(*index)?;
                    stack.push(value);
                }
                BinaryInstr::LoadGlobal(slot) => {
                    let value = self
                        .globals
                        .get(*slot as usize)
                        .cloned()
                        .ok_or_else(|| format!("invalid global slot: {}", slot))?;
                    stack.push(value);
                }
                BinaryInstr::StoreGlobal(slot) => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    let target = self
                        .globals
                        .get_mut(*slot as usize)
                        .ok_or_else(|| format!("invalid global slot: {}", slot))?;
                    *target = value.clone();
                    stack.push(value);
                }
                BinaryInstr::LoadLocal(slot) => {
                    let value = frame
                        .locals
                        .get(*slot as usize)
                        .cloned()
                        .ok_or_else(|| format!("invalid local slot: {}", slot))?;
                    stack.push(value);
                }
                BinaryInstr::StoreLocal(slot) => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    let target = frame
                        .locals
                        .get_mut(*slot as usize)
                        .ok_or_else(|| format!("invalid local slot: {}", slot))?;
                    *target = value.clone();
                    stack.push(value);
                }
                BinaryInstr::Add => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "add")?);
                }
                BinaryInstr::Sub => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "sub")?);
                }
                BinaryInstr::Mul => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "mul")?);
                }
                BinaryInstr::Div => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "div")?);
                }
                BinaryInstr::Mod => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "mod")?);
                }
                BinaryInstr::Pow => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "pow")?);
                }
                BinaryInstr::Eq => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "eq")?);
                }
                BinaryInstr::Neq => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "neq")?);
                }
                BinaryInstr::Lt => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "lt")?);
                }
                BinaryInstr::Lte => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "lte")?);
                }
                BinaryInstr::Gt => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "gt")?);
                }
                BinaryInstr::Gte => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(compare_op(left, right, "gte")?);
                }
                BinaryInstr::And => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "and")?);
                }
                BinaryInstr::Or => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "or")?);
                }
                BinaryInstr::Xor => {
                    let (left, right) = pop2(&mut stack)?;
                    stack.push(binary_op(left, right, "xor")?);
                }
                BinaryInstr::Neg => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match value {
                        Value::Number(n) => stack.push(Value::Number(-n)),
                        _ => return Err("NEG expects number".into()),
                    }
                }
                BinaryInstr::Not => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match value {
                        Value::Bool(v) => stack.push(Value::Bool(!v)),
                        _ => return Err("NOT expects bool".into()),
                    }
                }
                BinaryInstr::MakeList(len) => {
                    let mut items = Vec::with_capacity(*len as usize);
                    for _ in 0..*len {
                        items.push(stack.pop().ok_or_else(|| "stack underflow".to_string())?);
                    }
                    items.reverse();
                    stack.push(Value::List(items));
                }
                BinaryInstr::Pop => {
                    let _ = stack.pop();
                }
                BinaryInstr::Call { target, argc } => {
                    let mut args = Vec::with_capacity(*argc as usize);
                    for _ in 0..*argc {
                        args.push(stack.pop().ok_or_else(|| "stack underflow".to_string())?);
                    }
                    args.reverse();
                    stack.push(self.call(*target, args)?);
                }
                BinaryInstr::Jump(target) => {
                    ip = *target as usize;
                    continue;
                }
                BinaryInstr::JumpIfFalse(target) => {
                    let condition = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    match condition {
                        Value::Bool(false) => {
                            ip = *target as usize;
                            continue;
                        }
                        Value::Bool(true) => {}
                        _ => return Err("condition must be bool".into()),
                    }
                }
                BinaryInstr::SetupLoop(slot) => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    let target = frame
                        .loop_states
                        .get_mut(*slot as usize)
                        .ok_or_else(|| format!("invalid loop slot: {}", slot))?;
                    match value {
                        Value::List(values) => *target = Some((values, 0)),
                        _ => return Err("for loop expects list".into()),
                    }
                }
                BinaryInstr::ForIter {
                    loop_slot,
                    var_slot,
                    end_ip,
                } => {
                    let state = frame
                        .loop_states
                        .get_mut(*loop_slot as usize)
                        .ok_or_else(|| format!("invalid loop slot: {}", loop_slot))?;
                    let next_value = match state {
                        Some((values, index)) if *index < values.len() => {
                            let value = values[*index].clone();
                            *index += 1;
                            Some(value)
                        }
                        Some(_) => None,
                        None => return Err(format!("loop state not initialized: {}", loop_slot)),
                    };

                    if let Some(value) = next_value {
                        let target = frame
                            .locals
                            .get_mut(*var_slot as usize)
                            .ok_or_else(|| format!("invalid local slot: {}", var_slot))?;
                        *target = value;
                    } else {
                        ip = *end_ip as usize;
                        continue;
                    }
                }
                BinaryInstr::Return => return Ok(stack.pop().unwrap_or(Value::Void)),
                BinaryInstr::Halt => return Ok(stack.pop().unwrap_or(Value::Void)),
            }
            ip += 1;
        }

        Ok(stack.pop().unwrap_or(Value::Void))
    }

    fn call(&mut self, target: BinaryCallTarget, args: Vec<Value>) -> Result<Value, String> {
        match target {
            BinaryCallTarget::Builtin(BUILTIN_PRINT) => {
                let output = args
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{}", output);
                Ok(Value::Void)
            }
            BinaryCallTarget::Builtin(BUILTIN_LEN) => {
                if args.len() != 1 {
                    return Err("len() takes exactly one argument".into());
                }
                match &args[0] {
                    Value::List(values) => Ok(Value::Number(Fraction::from(values.len()))),
                    Value::String(s) => Ok(Value::Number(Fraction::from(s.len()))),
                    _ => Err("len() requires list or string".into()),
                }
            }
            BinaryCallTarget::Builtin(BUILTIN_RANGE) => builtin_range(args),
            BinaryCallTarget::Builtin(index) => Err(format!("unknown builtin target: {}", index)),
            BinaryCallTarget::Function(index) => {
                let function = self
                    .program
                    .functions
                    .get(index as usize)
                    .cloned()
                    .ok_or_else(|| format!("missing binary function index: {}", index))?;
                if args.len() != function.arg_count as usize {
                    return Err(format!(
                        "argument length mismatch for function index {}",
                        index
                    ));
                }
                let mut frame = BinaryFrame {
                    locals: vec![Value::Void; function.local_count as usize],
                    loop_states: vec![None; function.loop_count as usize],
                };
                for (slot, arg) in args.into_iter().enumerate() {
                    frame.locals[slot] = arg;
                }
                self.execute(&function.code, &mut frame)
            }
        }
    }

    fn constant(&self, index: u32) -> Result<&BinaryConst, String> {
        self.program
            .constants
            .get(index as usize)
            .ok_or_else(|| format!("invalid constant index: {}", index))
    }

    fn constant_to_value(&self, index: u32) -> Result<Value, String> {
        match self.constant(index)? {
            BinaryConst::Number(value) => Ok(Value::Number(value.clone())),
            BinaryConst::String(value) => Ok(Value::String(value.clone())),
            BinaryConst::Bool(value) => Ok(Value::Bool(*value)),
            BinaryConst::Void => Ok(Value::Void),
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
