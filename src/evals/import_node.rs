use crate::ast::ASTNode;
use crate::environment::Env;
use crate::environment::ExportedSymbolType;
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;

pub fn import_node(
    module_name: String,
    symbols: Vec<String>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let module_path = format!("{}.sag", module_name);
    match env.register_module(&module_name, &module_path) {
        Ok(_) => {}
        Err(e) => {
            return Err(RuntimeError::new(
                format!("Failed to import module {}: {:?}", module_name, e).as_str(),
                line,
                column,
            ))
        }
    }

    if let Some(module_env) = env.clone().get_module(&module_name) {
        for symbol in symbols {
            if let Some(exported_symbol_type) = module_env.get_exported_symbol(&symbol) {
                match exported_symbol_type.clone() {
                    ExportedSymbolType::Function => {
                        match module_env.clone().get_function(&symbol) {
                            Some(func) => {
                                env.register_function(symbol.clone(), func.clone());
                            }
                            None => {}
                        };
                    }
                    ExportedSymbolType::Struct => match module_env.clone().get_struct(&symbol) {
                        Some(s) => {
                            env.register_struct(s.clone())?;
                        }
                        None => {}
                    },
                    ExportedSymbolType::Variable => match module_env.get(&symbol, None) {
                        Some(symbol_value) => {
                            let _ = env.set(
                                symbol.clone(),
                                symbol_value.value.clone(),
                                symbol_value.variable_type.clone(),
                                symbol_value.value_type.clone(),
                                true,
                            );
                        }
                        None => {}
                    },
                };
            } else {
                return Err(RuntimeError::new(
                    format!("Symbol {} not found in module {}", symbol, module_name).as_str(),
                    line,
                    column,
                ));
            }
        }
    }
    Ok(Value::Void)
}

pub fn public_node(
    node: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    match *node.clone() {
        ASTNode::Function { name, .. } => {
            eval(*node, env)?;
            env.register_exported_symbol(name);
        }
        ASTNode::Struct { name, .. } => {
            eval(*node, env)?;
            env.register_exported_symbol(name);
        }
        ASTNode::Assign { name, .. } => {
            eval(*node, env)?;
            env.register_exported_symbol(name);
        }
        _ => {
            return Err(RuntimeError::new(
                format!("Only variables, struct and functions can be exported").as_str(),
                line,
                column,
            ))
        }
    }
    Ok(Value::Void)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ASTNode;
    use crate::evals::eval;

    #[test]
    fn test_import() {
        let mut env = Env::new();
        let file_path = "test_foo.sag";
        let _ = std::fs::write(file_path, "pub val a = 0\npub fun f() {{\n}}\npub struct Ham {\nx: number\n}\nimpl Ham {\n fun egg(self) {\n }\n }");
        let ast = ASTNode::Import {
            module_name: "test_foo".to_string(),
            symbols: vec!["a".to_string(), "f".to_string(), "Ham".to_string()],
            line: 0,
            column: 0,
        };
        assert_eq!(Value::Void, eval(ast, &mut env).unwrap());
        let module = env.get_module(&"test_foo".to_string()).unwrap();
        assert_eq!(
            match module.get_exported_symbol(&"a".to_string()) {
                Some(_) => true,
                None => false,
            },
            true
        );
        assert_eq!(
            match module.get_exported_symbol(&"f".to_string()) {
                Some(_) => true,
                None => false,
            },
            true
        );
        assert_eq!(
            match module.get_exported_symbol(&"Ham".to_string()) {
                Some(_) => true,
                None => false,
            },
            true
        );
        let _ = std::fs::remove_file(file_path);
    }
}
