use crate::{Environment, Evaluator, Object, RuntimeErrorKind};

pub fn handle_and_op(
    left: Object,
    right: Object,
    env: &mut Environment,
    evaluator: &mut Evaluator,
) -> Result<Object, RuntimeErrorKind> {
    match (left, right) {
        (Object::Integer(x), Object::Integer(y)) => Ok(Object::Integer(x.and(y, env, evaluator)?)),
        (x, y) => {
            let err=  RuntimeErrorKind::UnstructuredError{span : Default::default(), message : format!("currently we only support bitwise operations on ranged operations, found types {} and {}", x.r#type(), y.r#type())};
            Err(err)
        }
    }
}
