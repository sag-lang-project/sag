use fraction::Fraction;

use super::{BinaryCallTarget, BinaryConst, BinaryInstr, BinaryProgram};
use crate::compiler::{binary_op, builtin_range, compare_op, pop2};
use crate::value::Value;

const BUILTIN_PRINT: u8 = 0;
const BUILTIN_LEN: u8 = 1;
const BUILTIN_RANGE: u8 = 2;

struct BinaryFrame {
    locals: Vec<Value>,
    loop_states: Vec<Option<(Vec<Value>, usize)>>,
}

pub(crate) struct BinaryVm {
    program: BinaryProgram,
    globals: Vec<Value>,
}

impl BinaryVm {
    pub(crate) fn new(program: BinaryProgram) -> Self {
        let globals = vec![Value::Void; program.globals_count as usize];
        Self { program, globals }
    }

    pub(crate) fn run(&mut self) -> Result<(), String> {
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
                BinaryInstr::Const(index) => stack.push(self.constant_to_value(*index)?),
                BinaryInstr::LoadGlobal(slot) => {
                    let value = self.globals.get(*slot as usize).cloned().ok_or_else(|| {
                        format!("invalid global slot: {}", slot)
                    })?;
                    stack.push(value);
                }
                BinaryInstr::StoreGlobal(slot) => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    let target = self.globals.get_mut(*slot as usize).ok_or_else(|| {
                        format!("invalid global slot: {}", slot)
                    })?;
                    *target = value.clone();
                    stack.push(value);
                }
                BinaryInstr::LoadLocal(slot) => {
                    let value = frame.locals.get(*slot as usize).cloned().ok_or_else(|| {
                        format!("invalid local slot: {}", slot)
                    })?;
                    stack.push(value);
                }
                BinaryInstr::StoreLocal(slot) => {
                    let value = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
                    let target = frame.locals.get_mut(*slot as usize).ok_or_else(|| {
                        format!("invalid local slot: {}", slot)
                    })?;
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
                    let target = frame.loop_states.get_mut(*slot as usize).ok_or_else(|| {
                        format!("invalid loop slot: {}", slot)
                    })?;
                    match value {
                        Value::List(values) => *target = Some((values, 0)),
                        _ => return Err("for loop expects list".into()),
                    }
                }
                BinaryInstr::ForIter { loop_slot, var_slot, end_ip } => {
                    let state = frame.loop_states.get_mut(*loop_slot as usize).ok_or_else(|| {
                        format!("invalid loop slot: {}", loop_slot)
                    })?;
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
                        let target = frame.locals.get_mut(*var_slot as usize).ok_or_else(|| {
                            format!("invalid local slot: {}", var_slot)
                        })?;
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
                let output = args.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(" ");
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
                    return Err(format!("argument length mismatch for function index {}", index));
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
