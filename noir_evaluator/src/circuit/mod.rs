pub mod gate;

use super::polynomial::{Arithmetic, Polynomial};
use super::FieldElement;
use gate::Gate;

#[derive(Clone)]
pub struct Circuit(pub Vec<Gate>);

impl Circuit {}

// Each witness can be identified with an String
// The string will be derived from the identifier name in the
// variable
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct Witness(pub String, pub usize);

use std::cmp::Ordering;
impl Ord for Witness {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

impl Default for Witness {
    // Place holder value
    fn default() -> Witness {
        Witness("zero".to_string(), 0)
    }
}

impl Witness {
    pub fn new(variable_name: String, witness_index: usize) -> Witness {
        Witness(variable_name, witness_index)
    }
    pub fn witness_index(&self) -> usize {
        self.1
    }
    pub fn variable_name(&self) -> &str {
        &self.0
    }
}

// (selector_id, selector as an i128 , We don't have big int yet)
#[derive(Clone, Debug)]
pub struct Selector(pub String, pub Polynomial); //XXX(med) I guess we know it's going to be a FieldElement, so we should probably find a way to give it FieldElement directly instead of Polynomial

impl Default for Selector {
    fn default() -> Selector {
        Selector(
            "zero".to_string(),
            Polynomial::Constants(FieldElement::zero()),
        )
    }
}
