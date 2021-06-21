use noirc_errors::Span;
use noirc_frontend::hir_def::expr::HirCallExpression;

use super::BuiltInCaller;
use crate::binary_op;
use crate::errors::RuntimeError;
use crate::object::{Array, Object};
use crate::{Environment, Evaluator};

/// Takes the direct product of the elements in an array
pub struct ArrayProd;

impl BuiltInCaller for ArrayProd {
    fn call(
        evaluator: &mut Evaluator,
        env: &mut Environment,
        call_expr_span: (HirCallExpression, Span),
    ) -> Result<Object, RuntimeError> {
        let (mut call_expr, span) = call_expr_span;
        let arr_expr = {
            assert_eq!(call_expr.arguments.len(), 1);
            call_expr.arguments.pop().unwrap()
        };

        // ArrayProd should only take a single parameter, which is an array. This should have been caught by the compiler in the analysis phase
        let arr = Array::from_expression(evaluator, env, &arr_expr)?;

        let mut result = arr.get(0).map_err(|kind| kind.add_span(span))?;
        for i in 1..arr.contents.len() {
            result = binary_op::handle_mul_op(
                result,
                arr.get(i as u128).map_err(|kind| kind.add_span(span))?,
                evaluator,
            )
            .map_err(|kind| kind.add_span(span))?;
        }

        Ok(result)
    }
}