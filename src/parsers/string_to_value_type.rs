use crate::environment::ValueType;
use crate::parsers::Parser;
use crate::token::TokenKind;

impl Parser {
    pub fn string_to_value_type(&mut self, type_name: String) -> ValueType {
        let scope = self.get_current_scope();
        if let Some(struct_value) = self.get_struct(scope, type_name.clone()) {
            return struct_value;
        }

        match type_name.as_str() {
            "number" => ValueType::Number,
            "string" => ValueType::String,
            "bool" => ValueType::Bool,
            "void" => ValueType::Void,
            "List" => {
                self.extract_token(TokenKind::Lt);
                let element_type = match self.get_current_token() {
                    Some(token) => {
                        if let TokenKind::Identifier(type_name) = &token.kind {
                            self.consume_token();
                            self.string_to_value_type(type_name.clone())
                        } else {
                            panic!("expected type name");
                        }
                    }
                    None => panic!("expected type name"),
                };
                self.extract_token(TokenKind::Gt);
                ValueType::List(Box::new(element_type))
            }
            _ => panic!("undefined type: {:?}", type_name),
        }
    }
}
