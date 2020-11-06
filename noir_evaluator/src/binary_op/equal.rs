use super::sub::handle_sub_op;
use crate::{Environment, Evaluator, Gate, Linear, Object, EvaluatorError};

/// XXX(med) : So at the moment, Equals is the same as SUB
/// Most likely we will need to check if it is a predicate equal or infix equal

/// This calls the sub op under the hood
/// We negate the RHS and send it to the add op
pub fn handle_equal_op(
    left: Object,
    right: Object,
    env: &mut Environment,
    evaluator: &mut Evaluator,
) -> Result<Object, EvaluatorError> {

    let left_type = left.r#type();
    let right_type = right.r#type();

    let result = handle_sub_op(left, right, env, evaluator)?;

    match result {
        Object::Null => return Err(EvaluatorError::UnstructuredError{span : Default::default(), message : format!("constrain statement cannot output a null polynomial")}), // XXX; This should be BUG  severity as sub should have caught it
        Object::Constants(_) => return Err(EvaluatorError::UnstructuredError{span : Default::default(), message : format!("cannot constrain two constants")}),
        Object::Linear(linear) => evaluator.gates.push(Gate::Arithmetic(linear.into())),
        Object::Arithmetic(arith) => evaluator.gates.push(Gate::Arithmetic(arith)),
        Object::Integer(integer) => {
            let witness_linear = Linear::from_witness(integer.witness);

            evaluator
                .gates
                .push(Gate::Arithmetic(witness_linear.into()))
        }
        x => {
            return Err(EvaluatorError::UnsupportedOp{span : Default::default(), op : "equal".to_owned(), first_type : left_type.to_owned(), second_type :right_type.to_owned()});
        }
    }
    Ok(Object::Null)
}
