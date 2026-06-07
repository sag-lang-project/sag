use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use crate::ast::ASTNode;
use crate::value::Value;
use crate::environment::{Env, ValueType, EnvVariableType, EnvVariableValueInfo, VariableKeyInfo, FunctionInfo, ExportedSymbolType, MethodInfo};
use crate::evals::runtime_error::RuntimeError;
use crate::tokenizer::tokenize;
use crate::parsers::Parser;
use crate::evals::evals;
use crate::builtin::register_builtins;

/// A shared environment implementation that uses Rc and RefCell to avoid excessive cloning
#[derive(Debug, Clone)]
pub struct SharedEnv {
    inner: Rc<RefCell<SharedEnvInner>>,
}

#[derive(Debug)]
struct SharedEnvInner {
    variable_map: HashMap<VariableKeyInfo, EnvVariableValueInfo>,
    scope_stack: Vec<String>,
    functions: HashMap<String, FunctionInfo>,
    structs: HashMap<String, Value>,
    builtins: HashMap<String, FunctionInfo>,
    modules: HashMap<String, SharedEnv>,
    exported_symbols: HashMap<String, ExportedSymbolType>,
}

impl SharedEnv {
    pub fn new() -> Self {
        let mut env = SharedEnv {
            inner: Rc::new(RefCell::new(SharedEnvInner {
                variable_map: HashMap::new(),
                scope_stack: vec!["global".to_string()],
                functions: HashMap::new(),
                structs: HashMap::new(),
                builtins: HashMap::new(),
                modules: HashMap::new(),
                exported_symbols: HashMap::new(),
            })),
        };
        env
    }
    
    pub fn get(&self, name: &String, value_type: Option<&ValueType>) -> Option<EnvVariableValueInfo> {
        let inner = self.inner.borrow();
        
        // Try to find in current scope first
        for scope in inner.scope_stack.iter().rev() {
            let key = VariableKeyInfo {
                name: name.clone(),
                scope: scope.clone(),
            };
            
            if let Some(value_info) = inner.variable_map.get(&key) {
                if let Some(vt) = value_type {
                    if value_info.value_type != *vt {
                        continue;
                    }
                }
                return Some(value_info.clone());
            }
        }
        
        // Not found
        None
    }
    
    pub fn set(&mut self, name: String, value: Value, variable_type: EnvVariableType, value_type: ValueType, is_new: bool) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();
        let latest_scope = match inner.scope_stack.last() {
            Some(scope) => scope.clone(),
            None => return Err("Missing scope".into()),
        };
        
        // If it's a new variable, just insert it
        if is_new {
            inner.variable_map.insert(
                VariableKeyInfo {
                    name: name.clone(),
                    scope: latest_scope,
                },
                EnvVariableValueInfo {
                    value,
                    variable_type,
                    value_type,
                },
            );
            return Ok(());
        }
        
        // Check local scope variables and update if found
        let key = VariableKeyInfo {
            name: name.clone(),
            scope: latest_scope.clone(),
        };
        
        if inner.variable_map.contains_key(&key) {
            let value_info = inner.variable_map.get(&key).unwrap();
            if value_info.variable_type == EnvVariableType::Immutable {
                return Err("Cannot reassign to immutable variable".into());
            }
            
            inner.variable_map.insert(
                key,
                EnvVariableValueInfo {
                    value,
                    variable_type,
                    value_type,
                },
            );
            return Ok(());
        }
        
        // Check global scope variables and update if found
        let global_key = VariableKeyInfo {
            name: name.clone(),
            scope: "global".to_string(),
        };
        
        if inner.variable_map.contains_key(&global_key) {
            let value_info = inner.variable_map.get(&global_key).unwrap();
            if value_info.variable_type == EnvVariableType::Immutable {
                return Err("Cannot reassign to immutable variable".into());
            }
            
            inner.variable_map.insert(
                global_key,
                EnvVariableValueInfo {
                    value,
                    variable_type,
                    value_type,
                },
            );
            return Ok(());
        }
        
        // Not found anywhere, add as a new variable in the current scope
        inner.variable_map.insert(
            VariableKeyInfo {
                name,
                scope: latest_scope,
            },
            EnvVariableValueInfo {
                value,
                variable_type,
                value_type,
            },
        );
        
        Ok(())
    }
    
    pub fn enter_scope(&mut self, scope: String) {
        let mut inner = self.inner.borrow_mut();
        inner.scope_stack.push(scope);
    }
    
    pub fn leave_scope(&mut self) {
        let mut inner = self.inner.borrow_mut();
        if inner.scope_stack.len() == 1 && inner.scope_stack[0] == "global".to_string() {
            return;
        }
        inner.scope_stack.pop();
    }
    
    pub fn get_current_scope(&self) -> String {
        let inner = self.inner.borrow();
        match inner.scope_stack.last() {
            Some(scope) => scope.clone(),
            None => "global".to_string(),
        }
    }
    
    pub fn register_function(&mut self, name: String, function: FunctionInfo) {
        let mut inner = self.inner.borrow_mut();
        inner.functions.insert(name, function);
    }
    
    pub fn get_function(&self, name: &String) -> Option<FunctionInfo> {
        let inner = self.inner.borrow();
        inner.functions.get(name).cloned()
    }
    
    pub fn register_struct(&mut self, struct_value: Value) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let name = match &struct_value {
            Value::Struct { name, .. } => name.as_ref().clone(),
            _ => { return Err(RuntimeError::new("Invalid struct value", 0, 0)); }
        };
        
        if inner.structs.contains_key(&name) {
            return Err(RuntimeError::new(format!("Struct '{}' already exists", name).as_str(), 0, 0));
        }
        
        inner.structs.insert(name, struct_value);
        Ok(())
    }
    
    pub fn get_struct(&self, name: &String) -> Option<Value> {
        let inner = self.inner.borrow();
        inner.structs.get(name).cloned()
    }
    
    pub fn register_builtin(&mut self, name: String, function: fn(Vec<Value>) -> Value) {
        let mut inner = self.inner.borrow_mut();
        let function_info = FunctionInfo {
            arguments: vec![],
            return_type: ValueType::Any,
            body: None,
            builtin: Some(function),
        };
        inner.builtins.insert(name, function_info);
    }
    
    pub fn get_builtin(&self, name: &String) -> Option<FunctionInfo> {
        let inner = self.inner.borrow();
        inner.builtins.get(name).cloned()
    }
    
    pub fn register_module(&mut self, module_name: &String, module_path: &String) -> Result<(), String> {
        let inner_ref = self.inner.borrow();
        if inner_ref.modules.contains_key(module_name) {
            // Already registered
            return Ok(());
        }
        drop(inner_ref);
        
        let file_content = if !PathBuf::from(module_path).exists() {
            let module_path = PathBuf::from(format!("./.sag_packages/{}", module_path));
            if !module_path.exists() {
                return Err("missing package".to_string())
            }
            std::fs::read_to_string(module_path).unwrap()
        } else {
            std::fs::read_to_string(module_path).unwrap()
        };

        let tokens = tokenize(&file_content);
        let mut env = self.to_env();
        let builtins = register_builtins(&mut env);
        let mut parser = Parser::new(tokens, builtins);
        let ast_nodes = parser.parse_lines();
        if let Err(e) = ast_nodes {
            return Err(format!("Error: {:?}", e));
        }

        let mut module_env = SharedEnv::new();
        let result = evals(ast_nodes.unwrap(), &mut module_env.to_env());
        if let Err(e) = result {
            return Err(format!("Error: {:?}", e));
        }
        
        let mut inner = self.inner.borrow_mut();
        inner.modules.insert(module_name.to_string(), module_env);
        Ok(())
    }
    
    pub fn get_module(&self, module_name: &String) -> Option<SharedEnv> {
        let inner = self.inner.borrow();
        inner.modules.get(module_name).cloned()
    }
    
    pub fn register_exported_symbol(&mut self, name: String, symbol_type: ExportedSymbolType) {
        let mut inner = self.inner.borrow_mut();
        inner.exported_symbols.insert(name, symbol_type);
    }
    
    pub fn get_exported_symbol(&self, name: &String) -> Option<ExportedSymbolType> {
        let inner = self.inner.borrow();
        inner.exported_symbols.get(name).cloned()
    }
    
    pub fn update_global_env(&mut self, local_env: &Self) {
        let mut inner = self.inner.borrow_mut();
        let local_inner = local_env.inner.borrow();
        
        for (local_key, local_value) in &local_inner.variable_map {
            if local_key.scope == "global" && inner.variable_map.contains_key(local_key) {
                inner.variable_map.insert(local_key.clone(), local_value.clone());
            }
        }
    }
    
    // Convert to traditional Env for compatibility with existing code
    pub fn to_env(&self) -> Env {
        let inner = self.inner.borrow();
        let mut env = Env::new();
        
        // Copy variable map
        for (key, value) in &inner.variable_map {
            env.set(
                key.name.clone(),
                value.value.clone(),
                value.variable_type.clone(),
                value.value_type.clone(),
                true, // is_new
            ).unwrap();
        }
        
        // Copy functions
        for (name, function) in &inner.functions {
            env.register_function(name.clone(), function.clone());
        }
        
        // Copy structs
        for (name, struct_value) in &inner.structs {
            env.register_struct(struct_value.clone()).unwrap();
        }
        
        // Copy builtins
        for (name, function) in &inner.builtins {
            if let Some(builtin_fn) = function.builtin {
                env.register_builtin(name.clone(), builtin_fn);
            }
        }
        
        // Copy scope stack
        for scope in &inner.scope_stack {
            if scope != "global" {
                env.enter_scope(scope.clone());
            }
        }
        
        // Copy exported symbols
        for (name, symbol_type) in &inner.exported_symbols {
            env.register_exported_symbol(name.clone());
        }
        
        // Copy modules (simplified - in a real implementation you'd need to handle this recursively)
        // This is a limitation of the current implementation
        
        env
    }
    
    // Create from traditional Env for compatibility with existing code
    pub fn from_env(env: &Env) -> Self {
        let mut shared_env = SharedEnv::new();
        let mut inner = shared_env.inner.borrow_mut();
        
        // Copy variable map
        for (key, value) in &env.variable_map {
            inner.variable_map.insert(key.clone(), value.clone());
        }
        
        // Copy functions
        for (name, function) in &env.functions {
            inner.functions.insert(name.clone(), function.clone());
        }
        
        // Copy structs
        for (name, struct_value) in &env.structs {
            inner.structs.insert(name.clone(), struct_value.clone());
        }
        
        // Copy builtins
        for (name, function) in &env.builtins {
            inner.builtins.insert(name.clone(), function.clone());
        }
        
        // Copy scope stack
        inner.scope_stack = env.scope_stack.clone();
        
        // Copy exported symbols
        for (name, symbol_type) in &env.exported_symbols {
            inner.exported_symbols.insert(name.clone(), symbol_type.clone());
        }
        
        // Copy modules (simplified - in a real implementation you'd need to handle this recursively)
        // This is a limitation of the current implementation
        
        drop(inner);
        shared_env
    }
}

// Implement PartialEq for SharedEnv to compare environments
impl PartialEq for SharedEnv {
    fn eq(&self, other: &Self) -> bool {
        // Compare Rc pointers for efficiency
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
