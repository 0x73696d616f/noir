use super::errors::RuntimeErrorKind;
use super::object::{Array, Object};
use noirc_frontend::hir::scope::{
    ScopeForest as GenericScopeForest, ScopeTree as GenericScopeTree,
};

type ScopeTree = GenericScopeTree<String, Object>;
type ScopeForest = GenericScopeForest<String, Object>;

pub struct Environment(ScopeForest);

impl Environment {
    pub fn new() -> Environment {
        Environment(ScopeForest::new())
    }

    pub fn start_function_environment(&mut self) {
        self.0.start_function()
    }
    pub fn end_function_environment(&mut self) -> ScopeTree {
        self.0.end_function()
    }

    pub fn start_for_loop(&mut self) {
        self.0.start_for_loop()
    }

    pub fn end_for_loop(&mut self) {
        self.0.end_for_loop();
    }

    pub fn store(&mut self, name: String, object: Object) {
        let scope = self.0.get_mut_scope();
        scope.add_key_value(name.clone(), object);
    }

    pub fn get(&mut self, name: &String) -> Object {
        let scope = self.0.current_scope_tree();
        scope.find_key(name).unwrap().clone()
    }
    pub fn get_array(&mut self, name: &String) -> Result<Array, RuntimeErrorKind> {
        let poly = self.get(name);

        match poly {
            Object::Array(arr) => Ok(arr),
            k => Err(RuntimeErrorKind::ArrayNotFound {
                name: name.to_owned(),
                found_type: k.r#type().to_owned(),
            }),
        }
    }
}
