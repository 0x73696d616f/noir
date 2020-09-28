/// This module handles all of the binary operations between polynomials
pub mod add;
pub mod and;
pub mod bound_check;
pub mod cast;
pub mod div;
pub mod equal;
pub mod mul;
pub mod neq;
pub mod sub;
pub mod xor;

pub use add::handle_add_op;
pub use and::handle_and_op;
pub use bound_check::handle_greater_than_equal_op;
pub use bound_check::handle_greater_than_op;
pub use bound_check::handle_less_than_equal_op;
pub use bound_check::handle_less_than_op;
pub use cast::handle_cast_op;
pub use div::handle_div_op;
pub use equal::handle_equal_op;
pub use mul::handle_mul_op;
pub use neq::handle_neq_op;
pub use sub::handle_sub_op;
pub use xor::handle_xor_op;

use crate::{Environment, Evaluator, FieldElement, Polynomial, Type};

/// Creates a new witness and constrains it to be the inverse of the polynomial passed in
pub fn invert(x: Polynomial, env: &mut Environment, evaluator: &mut Evaluator) -> Polynomial {
    // Create a fresh witness
    // XXX: We need to create a better function for fresh variables
    let inter_var_name = format!("{}{}", "inverse_", evaluator.get_unique_value(),);
    evaluator.store_witness(inter_var_name.clone(), Type::Witness);
    let x_inv = evaluator.store_lone_variable(inter_var_name, env);

    // Multiply inverse by original value
    let should_be_one = handle_mul_op(x, x_inv.clone(), env, evaluator);

    // Constrain x * x_inv = 1
    let _ = handle_equal_op(
        should_be_one,
        Polynomial::Constants(FieldElement::one()),
        env,
        evaluator,
    );

    // Return inverse
    x_inv
}

fn unsupported_error(polynomials: Vec<Polynomial>) -> Polynomial {
    panic!(
        "We currently do not support operations involving {:?}",
        polynomials
    )
}
