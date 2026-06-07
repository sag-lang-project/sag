use std::collections::HashMap;

use fraction::Fraction;

mod lowering;
mod vm;

const MAGIC_BINARY: &[u8; 4] = b"SAGB";
const BINARY_VERSION_MAJOR: u16 = 1;
const BINARY_VERSION_MINOR: u16 = 0;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum BinaryConst {
    Number(Fraction),
    String(String),
    Bool(bool),
    Void,
}

#[derive(Debug, Clone)]
pub(super) enum BinaryPseudoInstr {
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
pub(super) enum BinaryInstr {
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
pub(super) struct BinaryFunction {
    arg_count: u16,
    local_count: u16,
    loop_count: u16,
    code: Vec<BinaryInstr>,
}

#[derive(Debug, Clone)]
pub(super) struct BinaryProgram {
    constants: Vec<BinaryConst>,
    globals_count: u32,
    entry_local_count: u16,
    entry_loop_count: u16,
    entry: Vec<BinaryInstr>,
    functions: Vec<BinaryFunction>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BinaryCallTarget {
    Builtin(u8),
    Function(u16),
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

pub(super) use lowering::lower_program;
pub(super) use vm::BinaryVm;

pub(super) fn is_binary_format(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC_BINARY)
}

pub(super) fn serialize_program_binary(program: &BinaryProgram) -> Vec<u8> {
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

pub(super) fn parse_program_binary(bytes: &[u8]) -> Result<BinaryProgram, String> {
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

pub(super) fn resolve_binary_labels(code: Vec<BinaryPseudoInstr>) -> Result<Vec<BinaryInstr>, String> {
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
