use crate::ast::ASTNode;
use crate::builtin::register_builtins;
use crate::environment::{
    Env, EnvVariableType, ExportedSymbolType, FunctionInfo, ValueType, VariableKeyInfo,
};
use crate::evals::evals;
use crate::evals::runtime_error::RuntimeError;
use crate::parsers::Parser;
use crate::rc_value::RcValue;
use crate::tokenizer::tokenize;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Rc版の環境実装。参照カウントを使用してクローン操作のパフォーマンスを向上させる。
#[derive(Debug)]
pub struct RcEnv {
    inner: Rc<RefCell<RcEnvInner>>,
}

#[derive(Debug)]
struct RcEnvInner {
    variable_map: HashMap<VariableKeyInfo, RcEnvVariableValueInfo>,
    scope_stack: Vec<String>,
    functions: HashMap<String, FunctionInfo>,
    rc_functions: HashMap<String, RcFunctionInfo>,
    structs: HashMap<String, RcValue>,
    builtins: HashMap<String, FunctionInfo>,
    rc_builtins: HashMap<String, RcFunctionInfo>,
    modules: HashMap<String, RcEnv>,
    exported_symbols: HashMap<String, ExportedSymbolType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RcEnvVariableValueInfo {
    pub value: RcValue,
    pub variable_type: EnvVariableType,
    pub value_type: ValueType,
}

#[derive(Debug, Clone)]
pub struct RcFunctionInfo {
    pub arguments: Vec<ASTNode>,
    pub return_type: ValueType,
    pub body: Option<ASTNode>,
    pub builtin: Option<fn(Vec<RcValue>) -> RcValue>,
}

impl RcEnv {
    pub fn new() -> Self {
        RcEnv {
            inner: Rc::new(RefCell::new(RcEnvInner {
                variable_map: HashMap::new(),
                scope_stack: vec!["global".to_string()],
                functions: HashMap::new(),
                rc_functions: HashMap::new(),
                structs: HashMap::new(),
                builtins: HashMap::new(),
                rc_builtins: HashMap::new(),
                modules: HashMap::new(),
                exported_symbols: HashMap::new(),
            })),
        }
    }

    /// 通常のEnvからRcEnvに変換する（最適化版）
    pub fn from_env(env: &Env) -> Self {
        let mut rc_env = RcEnv::new();

        // 変数マップをコピー
        for (key, value) in env.get_variable_map() {
            let rc_value = RcValue::from_value(&value.value);
            rc_env
                .set(
                    key.name.clone(),
                    rc_value,
                    value.variable_type.clone(),
                    value.value_type.clone(),
                    true, // is_new
                )
                .unwrap();
        }

        // 関数をコピー
        {
            let mut inner = rc_env.inner.borrow_mut();
            for (name, function) in env.get_functions() {
                inner.functions.insert(name.clone(), function.clone());
            }

            // 構造体をコピー
            for (name, struct_value) in env.get_structs() {
                inner
                    .structs
                    .insert(name.clone(), RcValue::from_value(struct_value));
            }

            // ビルトイン関数をコピー
            for (name, function) in env.get_builtins() {
                inner.builtins.insert(name.clone(), function.clone());
            }

            // スコープスタックをコピー
            inner.scope_stack = env.get_scope_stack().clone();

            // エクスポートされたシンボルをコピー
            for (name, symbol_type) in env.get_exported_symbols() {
                inner
                    .exported_symbols
                    .insert(name.clone(), symbol_type.clone());
            }
        }

        // モジュールをコピー
        for (name, module_env) in env.get_modules() {
            let rc_module_env = RcEnv::from_env(module_env);
            let mut inner = rc_env.inner.borrow_mut();
            inner.modules.insert(name.clone(), rc_module_env);
        }

        rc_env
    }

    /// RcEnvから通常のEnvに変換する（最適化版）
    pub fn to_env(&self) -> Env {
        let mut env = Env::new();
        let inner = self.inner.borrow();

        // 変数マップをコピー
        for (key, value) in &inner.variable_map {
            let std_value = value.value.to_value();
            env.set(
                key.name.clone(),
                std_value,
                value.variable_type.clone(),
                value.value_type.clone(),
                true, // is_new
            )
            .unwrap();
        }

        // 関数をコピー
        for (name, function) in &inner.functions {
            env.register_function(name.clone(), function.clone());
        }

        // 構造体をコピー
        for (_name, struct_value) in &inner.structs {
            env.register_struct(struct_value.to_value()).unwrap();
        }

        // ビルトイン関数をコピー
        for (name, function) in &inner.builtins {
            if let Some(builtin_fn) = function.builtin {
                env.register_builtin(name.clone(), builtin_fn);
            }
        }

        // スコープスタックをコピー
        env.set_scope_stack(inner.scope_stack.clone());

        // エクスポートされたシンボルをコピー
        for (name, _symbol_type) in &inner.exported_symbols {
            env.register_exported_symbol(name.clone());
        }

        // モジュールをコピー
        for (name, module_env) in &inner.modules {
            env.insert_module(name.clone(), module_env.to_env());
        }

        env
    }

    pub fn get(
        &self,
        name: &String,
        value_type: Option<&ValueType>,
    ) -> Option<RcEnvVariableValueInfo> {
        let inner = self.inner.borrow();

        // 現在のスコープから順に検索
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

        None
    }

    pub fn set(
        &mut self,
        name: String,
        value: RcValue,
        variable_type: EnvVariableType,
        value_type: ValueType,
        is_new: bool,
    ) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();
        let latest_scope = match inner.scope_stack.last() {
            Some(scope) => scope.clone(),
            None => return Err("Missing scope".into()),
        };

        // 新規変数の場合はそのまま追加
        if is_new {
            inner.variable_map.insert(
                VariableKeyInfo {
                    name: name.clone(),
                    scope: latest_scope,
                },
                RcEnvVariableValueInfo {
                    value,
                    variable_type,
                    value_type,
                },
            );
            return Ok(());
        }

        // ローカルスコープの変数をチェックして更新
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
                RcEnvVariableValueInfo {
                    value,
                    variable_type,
                    value_type,
                },
            );
            return Ok(());
        }

        // グローバルスコープの変数をチェックして更新
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
                RcEnvVariableValueInfo {
                    value,
                    variable_type,
                    value_type,
                },
            );
            return Ok(());
        }

        // どこにも存在しない場合は現在のスコープに新規追加
        inner.variable_map.insert(
            VariableKeyInfo {
                name,
                scope: latest_scope,
            },
            RcEnvVariableValueInfo {
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

    pub fn register_rc_function(&mut self, name: String, function: RcFunctionInfo) {
        let mut inner = self.inner.borrow_mut();
        inner.rc_functions.insert(name, function);
    }

    pub fn get_function(&self, name: &String) -> Option<FunctionInfo> {
        let inner = self.inner.borrow();
        inner.functions.get(name).cloned()
    }

    pub fn get_rc_function(&self, name: &String) -> Option<RcFunctionInfo> {
        let inner = self.inner.borrow();
        inner.rc_functions.get(name).cloned()
    }

    pub fn register_struct(&mut self, struct_value: RcValue) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let name = match &struct_value {
            RcValue::Struct { name, .. } => name.to_string(),
            _ => {
                return Err(RuntimeError::new("Invalid struct value", 0, 0));
            }
        };

        if inner.structs.contains_key(&name) {
            return Err(RuntimeError::new(
                format!("Struct '{}' already exists", name).as_str(),
                0,
                0,
            ));
        }

        inner.structs.insert(name, struct_value);
        Ok(())
    }

    pub fn get_struct(&self, name: &String) -> Option<RcValue> {
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

    pub fn register_rc_builtin(&mut self, name: String, function: fn(Vec<RcValue>) -> RcValue) {
        let mut inner = self.inner.borrow_mut();
        let function_info = RcFunctionInfo {
            arguments: vec![],
            return_type: ValueType::Any,
            body: None,
            builtin: Some(function),
        };
        inner.rc_builtins.insert(name.clone(), function_info);
    }

    pub fn get_builtin(&self, name: &String) -> Option<FunctionInfo> {
        let inner = self.inner.borrow();
        inner.builtins.get(name).cloned()
    }

    pub fn get_rc_builtin(&self, name: &String) -> Option<RcFunctionInfo> {
        let inner = self.inner.borrow();
        inner.rc_builtins.get(name).cloned()
    }

    pub fn get_rc_builtins(&self) -> Vec<String> {
        let inner = self.inner.borrow();
        inner.rc_builtins.keys().cloned().collect()
    }

    pub fn register_module(
        &mut self,
        module_name: &String,
        module_path: &String,
    ) -> Result<(), String> {
        let inner_ref = self.inner.borrow();
        if inner_ref.modules.contains_key(module_name) {
            // 既に登録済み
            return Ok(());
        }
        drop(inner_ref);

        let file_content = if !PathBuf::from(module_path).exists() {
            let module_path = PathBuf::from(format!("./.sag_packages/{}", module_path));
            if !module_path.exists() {
                return Err("missing package".to_string());
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

        let mut module_env = Env::new();
        let result = evals(ast_nodes.unwrap(), &mut module_env);
        if let Err(e) = result {
            return Err(format!("Error: {:?}", e));
        }

        let rc_module_env = RcEnv::from_env(&module_env);
        let mut inner = self.inner.borrow_mut();
        inner.modules.insert(module_name.to_string(), rc_module_env);
        Ok(())
    }

    pub fn get_module(&self, module_name: &String) -> Option<RcEnv> {
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
                inner
                    .variable_map
                    .insert(local_key.clone(), local_value.clone());
            }
        }
    }

    // Copy-on-Write操作
    pub fn update_variable(&mut self, name: &String, value: RcValue) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();

        // 変数を検索
        for scope in inner.scope_stack.iter().rev() {
            let key = VariableKeyInfo {
                name: name.clone(),
                scope: scope.clone(),
            };

            if inner.variable_map.contains_key(&key) {
                let var_info = inner.variable_map.get(&key).unwrap().clone();
                if var_info.variable_type == EnvVariableType::Immutable {
                    return Err(format!("Cannot modify immutable variable: {}", name));
                }

                // 変数を更新
                inner.variable_map.insert(
                    key,
                    RcEnvVariableValueInfo {
                        value,
                        variable_type: var_info.variable_type,
                        value_type: var_info.value_type,
                    },
                );

                return Ok(());
            }
        }

        Err(format!("Variable not found: {}", name))
    }
}

// RcEnvの比較実装
impl PartialEq for RcEnv {
    fn eq(&self, other: &Self) -> bool {
        // Rcポインタを比較して効率化
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

// RcEnvのクローン実装
impl Clone for RcEnv {
    fn clone(&self) -> Self {
        // 単純にRcポインタをクローンするのではなく、内部の状態もコピー
        let inner = self.inner.borrow();
        let new_inner = RcEnvInner {
            variable_map: inner.variable_map.clone(),
            scope_stack: inner.scope_stack.clone(),
            functions: inner.functions.clone(),
            rc_functions: inner.rc_functions.clone(),
            structs: inner.structs.clone(),
            builtins: inner.builtins.clone(),
            rc_builtins: inner.rc_builtins.clone(),
            modules: inner.modules.clone(),
            exported_symbols: inner.exported_symbols.clone(),
        };
        RcEnv {
            inner: Rc::new(RefCell::new(new_inner)),
        }
    }
}
