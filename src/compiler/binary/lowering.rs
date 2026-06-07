use std::collections::HashMap;

use super::{
    resolve_binary_labels, BinaryCallTarget, BinaryConst, BinaryFunction, BinaryProgram,
    BinaryPseudoInstr,
};
use crate::compiler::{Instr, Program};

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

const BUILTIN_PRINT: u8 = 0;
const BUILTIN_LEN: u8 = 1;
const BUILTIN_RANGE: u8 = 2;

pub(crate) fn lower_program(program: &Program) -> Result<BinaryProgram, String> {
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
    let entry = resolve_binary_labels(lower_entry_code(&program.entry, &mut lowering, &mut entry_ctx)?)?;

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
        let code =
            resolve_binary_labels(lower_function_code(&function.code, &mut lowering, &mut ctx)?)?;
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
                return Err(format!("undefined global variable during binary lowering: {}", name));
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
                return Err(format!("undefined variable during function binary lowering: {}", name));
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
