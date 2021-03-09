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

use crate::{Evaluator, FieldElement, Object, RuntimeErrorKind};

/// Creates a new witness and constrains it to be the inverse of the polynomial passed in
pub fn invert(x: Object, evaluator: &mut Evaluator) -> Result<Object, RuntimeErrorKind> {
    // Create a fresh witness

    let inverse_witness = evaluator.add_witness_to_cs();
    let inverse_obj = Object::from_witness(inverse_witness);

    // Multiply inverse by original value
    let should_be_one = handle_mul_op(x, inverse_obj.clone(), evaluator)?;

    // Constrain x * x_inv = 1
    let _ = handle_equal_op(
        should_be_one,
        Object::Constants(FieldElement::one()),
        evaluator,
    );

    // Return inverse
    Ok(inverse_obj)
}
