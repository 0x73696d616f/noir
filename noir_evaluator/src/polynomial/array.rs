use crate::polynomial::Polynomial;
use crate::{Environment, Evaluator};
use noirc_frontend::ast::ArrayLiteral;

#[derive(Clone, Debug)]
pub struct Array {
    pub contents: Vec<Polynomial>,
    pub length: u128,
}

impl Array {
    pub fn from(evaluator: &mut Evaluator, env: &mut Environment, arr_lit: ArrayLiteral) -> Array {
        // Take each element in the array and turn it into an object
        // XXX: We do not do any type checking here, this will be done by the analyser.
        // It will ensure that each type is the same and that the ArrayLiteral has an appropriate type
        let elements_as_objects: Vec<_> = arr_lit
            .contents
            .into_iter()
            .map(|expr| evaluator.expression_to_polynomial(env, expr))
            .collect();
        Array {
            contents: elements_as_objects,
            length: arr_lit.length,
        }
    }
    pub fn get(&self, index: u128) -> Polynomial {
        if index >= self.length {
            panic!(
                "out of bounds error, index is {} but length is {}",
                index, self.length
            )
        };

        self.contents[index as usize].clone()
    }
}
