// Aztec uses a `TurboFormat` object in order to bridge the gap between Rust and C++.
// This serialiser converts the IR into the `TurboFormat` which can then be fed into the WASM file
use crate::barretenberg_rs::composer::{
    Blake2sConstraint, Constraint, ConstraintSystem, HashToFieldConstraint, LogicConstraint,
    MerkleMembershipConstraint, MerkleRootConstraint, PedersenConstraint, RangeConstraint,
    SchnorrConstraint, Sha256Constraint,
};
use acir::circuit::{Circuit, Gate};
use acir::native_types::Arithmetic;
use acir::OPCODE;
use noir_field::FieldElement;

/// Converts an `IR` into the `StandardFormat` constraint system
pub fn serialise_circuit(circuit: &Circuit) -> ConstraintSystem {
    // Create constraint system
    let mut constraints: Vec<Constraint> = Vec::new();
    let mut range_constraints: Vec<RangeConstraint> = Vec::new();
    let mut logic_constraints: Vec<LogicConstraint> = Vec::new();
    let mut sha256_constraints: Vec<Sha256Constraint> = Vec::new();
    let mut blake2s_constraints: Vec<Blake2sConstraint> = Vec::new();
    let mut pedersen_constraints: Vec<PedersenConstraint> = Vec::new();
    let mut merkle_membership_constraints: Vec<MerkleMembershipConstraint> = Vec::new();
    let mut merkle_root_constraints: Vec<MerkleRootConstraint> = Vec::new();
    let mut schnorr_constraints: Vec<SchnorrConstraint> = Vec::new();
    let mut hash_to_field_constraints: Vec<HashToFieldConstraint> = Vec::new();

    for gate in circuit.gates.iter() {
        match gate {
            Gate::Arithmetic(arithmetic) => {
                let constraint = serialise_arithmetic_gates(arithmetic);
                constraints.push(constraint);
            }
            Gate::Range(witness, num_bits) => {
                let range_constraint = RangeConstraint {
                    a: witness.witness_index() as i32,
                    num_bits: *num_bits as i32,
                };
                range_constraints.push(range_constraint);
            }
            Gate::And(and_gate) => {
                let and = LogicConstraint::and(
                    and_gate.a.witness_index() as i32,
                    and_gate.b.witness_index() as i32,
                    and_gate.result.witness_index() as i32,
                    and_gate.num_bits as i32,
                );
                logic_constraints.push(and);
            }
            Gate::Xor(xor_gate) => {
                let xor = LogicConstraint::xor(
                    xor_gate.a.witness_index() as i32,
                    xor_gate.b.witness_index() as i32,
                    xor_gate.result.witness_index() as i32,
                    xor_gate.num_bits as i32,
                );
                logic_constraints.push(xor);
            }
            Gate::GadgetCall(gadget_call) => {
                match gadget_call.name {
                    OPCODE::SHA256 => {
                        let mut sha256_inputs: Vec<(i32, i32)> = Vec::new();
                        for input in gadget_call.inputs.iter() {
                            let witness_index = input.witness.witness_index() as i32;
                            let num_bits = input.num_bits as i32;
                            sha256_inputs.push((witness_index, num_bits));
                        }

                        assert_eq!(gadget_call.outputs.len(), 2);

                        let result_low_128_witness_index =
                            gadget_call.outputs[0].witness_index() as i32;
                        let result_high_128_witness_index =
                            gadget_call.outputs[1].witness_index() as i32;

                        let sha256_constraint = Sha256Constraint {
                            inputs: sha256_inputs,
                            result_low_128: result_low_128_witness_index,
                            result_high_128: result_high_128_witness_index,
                        };

                        sha256_constraints.push(sha256_constraint);
                    }
                    OPCODE::Blake2s => {
                        let mut blake2s_inputs: Vec<(i32, i32)> = Vec::new();
                        for input in gadget_call.inputs.iter() {
                            let witness_index = input.witness.witness_index() as i32;
                            let num_bits = input.num_bits as i32;
                            blake2s_inputs.push((witness_index, num_bits));
                        }

                        assert_eq!(gadget_call.outputs.len(), 2);

                        let result_low_128_witness_index =
                            gadget_call.outputs[0].witness_index() as i32;
                        let result_high_128_witness_index =
                            gadget_call.outputs[1].witness_index() as i32;

                        let blake2s_constraint = Blake2sConstraint {
                            inputs: blake2s_inputs,
                            result_low_128: result_low_128_witness_index,
                            result_high_128: result_high_128_witness_index,
                        };

                        blake2s_constraints.push(blake2s_constraint);
                    }
                    OPCODE::MerkleRoot => {
                        let mut leaves = Vec::new();
                        for input in gadget_call.inputs.iter() {
                            let witness_index = input.witness.witness_index() as i32;

                            // Ignore num_bits, it's only needed for
                            // hash functions like sha2/blake

                            leaves.push(witness_index);
                        }

                        assert_eq!(gadget_call.outputs.len(), 1);

                        let root_index = gadget_call.outputs[0].witness_index() as i32;

                        let constraint = MerkleRootConstraint {
                            leaves,
                            root: root_index,
                        };

                        merkle_root_constraints.push(constraint);
                    }
                    OPCODE::MerkleMembership => {
                        let mut inputs_iter = gadget_call.inputs.iter().peekable();

                        // root
                        let root = {
                            let root_input = inputs_iter.next().expect("missing merkle root");
                            root_input.witness.witness_index() as i32
                        };
                        // leaf
                        let leaf = {
                            let leaf_input = inputs_iter
                                .next()
                                .expect("missing leaf to check membership for");
                            leaf_input.witness.witness_index() as i32
                        };
                        // index
                        let index = {
                            let index_input = inputs_iter.next().expect("missing index for leaf");
                            index_input.witness.witness_index() as i32
                        };

                        if inputs_iter.peek().is_none() {
                            unreachable!("cannot check membership without a hash path")
                        }

                        let mut hash_path = Vec::new();
                        while let (Some(path_left), path_right_option) =
                            (inputs_iter.next(), inputs_iter.next())
                        {
                            let path_right = path_right_option
                                .expect("iterator contains an odd amount of items");

                            let path_left_index = path_left.witness.witness_index() as i32;
                            let path_right_index = path_right.witness.witness_index() as i32;

                            hash_path.push((path_left_index, path_right_index));
                        }

                        // result
                        let result = gadget_call.outputs[0].witness_index() as i32;

                        let constraint = MerkleMembershipConstraint {
                            root,
                            leaf,
                            index,
                            result,
                            hash_path,
                        };

                        merkle_membership_constraints.push(constraint);
                    }
                    OPCODE::SchnorrVerify => {
                        let mut inputs_iter = gadget_call.inputs.iter().peekable();

                        // pub_key_x
                        let public_key_x = {
                            let pub_key_x = inputs_iter
                                .next()
                                .expect("missing `x` component for public key");
                            pub_key_x.witness.witness_index() as i32
                        };
                        // pub_key_y
                        let public_key_y = {
                            let pub_key_y = inputs_iter
                                .next()
                                .expect("missing `y` component for public key");
                            pub_key_y.witness.witness_index() as i32
                        };
                        // signature
                        let mut signature = [0i32; 64];
                        for i in 0..64 {
                            let sig_byte = inputs_iter.next().expect(&format!(
                                "missing rest of signature. tried to get byte {} but failed",
                                i
                            ));
                            let sig_byte_index = sig_byte.witness.witness_index() as i32;
                            signature[i] = sig_byte_index
                        }

                        // The rest of the input is the message
                        let mut message = Vec::new();
                        while let Some(msg) = inputs_iter.next() {
                            let msg_byte_index = msg.witness.witness_index() as i32;
                            message.push(msg_byte_index);
                        }

                        assert!(inputs_iter.next().is_none());

                        // result
                        let result = gadget_call.outputs[0].witness_index() as i32;

                        let constraint = SchnorrConstraint {
                            message,
                            signature,
                            public_key_x,
                            public_key_y,
                            result,
                        };

                        schnorr_constraints.push(constraint);
                    }
                    OPCODE::AES => panic!("AES has not yet been implemented"),
                    OPCODE::Pedersen => {
                        let mut inputs = Vec::new();
                        for scalar in gadget_call.inputs.iter() {
                            let scalar_index = scalar.witness.witness_index() as i32;
                            inputs.push(scalar_index);
                        }

                        let result = gadget_call.outputs[0].witness_index() as i32;

                        let constraint = PedersenConstraint { inputs, result };

                        pedersen_constraints.push(constraint);
                    }
                    OPCODE::HashToField => {
                        let mut hash_to_field_inputs: Vec<(i32, i32)> = Vec::new();
                        for input in gadget_call.inputs.iter() {
                            let witness_index = input.witness.witness_index() as i32;
                            let num_bits = input.num_bits as i32;
                            hash_to_field_inputs.push((witness_index, num_bits));
                        }

                        assert_eq!(gadget_call.outputs.len(), 1);

                        let result = gadget_call.outputs[0].witness_index() as i32;

                        let hash_to_field_constraint = HashToFieldConstraint {
                            inputs: hash_to_field_inputs,
                            result,
                        };

                        hash_to_field_constraints.push(hash_to_field_constraint);
                    }
                };
            }
        }
    }

    // Create constraint system
    let constraint_system = ConstraintSystem {
        var_num: circuit.num_witnesses,
        public_inputs: circuit.public_inputs.indices(),
        logic_constraints,
        range_constraints,
        sha256_constraints,
        merkle_membership_constraints,
        merkle_root_constraints,
        pedersen_constraints,
        schnorr_constraints,
        blake2s_constraints,
        hash_to_field_constraints,
        constraints,
    };

    constraint_system
}

#[allow(non_snake_case)]
fn serialise_arithmetic_gates(gate: &Arithmetic) -> Constraint {
    let mut a: i32 = 0;
    let mut b: i32 = 0;
    let mut c: i32 = 0;
    let mut qm: FieldElement = 0.into();
    let mut ql: FieldElement = 0.into();
    let mut qr: FieldElement = 0.into();
    let mut qo: FieldElement = 0.into();
    let qc: FieldElement;

    // check mul gate
    if gate.mul_terms.len() != 0 {
        let mul_term = &gate.mul_terms[0];
        qm = mul_term.0;

        // Get wL term
        let wL = &mul_term.1;
        a = wL.witness_index() as i32;

        // Get wR term
        let wR = &mul_term.2;
        b = wR.witness_index() as i32;
    }

    // If there is only one simplified fan term,
    // then put it in qO * wO
    // This is incase, the qM term is non-zero
    if gate.linear_combinations.len() == 1 {
        let qO_wO_term = &gate.linear_combinations[0];
        qo = qO_wO_term.0;

        let wO = &qO_wO_term.1;
        c = wO.witness_index() as i32;
    }

    // XXX: THis is a code smell. Refactor to be better. Maybe change barretenberg to take vectors
    // If there is more than one term,
    // Then add normally
    if gate.linear_combinations.len() == 2 {
        let qL_wL_term = &gate.linear_combinations[0];
        ql = qL_wL_term.0;

        let wL = &qL_wL_term.1;
        a = wL.witness_index() as i32;

        let qR_wR_term = &gate.linear_combinations[1];
        qr = qR_wR_term.0;

        let wR = &qR_wR_term.1;
        b = wR.witness_index() as i32;
    }

    if gate.linear_combinations.len() == 3 {
        let qL_wL_term = &gate.linear_combinations[0];
        ql = qL_wL_term.0;

        let wL = &qL_wL_term.1;
        a = wL.witness_index() as i32;

        let qR_wR_term = &gate.linear_combinations[1];
        qr = qR_wR_term.0;

        let wR = &qR_wR_term.1;
        b = wR.witness_index() as i32;

        let qO_wO_term = &gate.linear_combinations[2];
        qo = qO_wO_term.0;

        let wO = &qO_wO_term.1;
        c = wO.witness_index() as i32;
    }

    // Add the qc term
    qc = gate.q_C;

    Constraint {
        a,
        b,
        c,
        qm,
        ql,
        qr,
        qo,
        qc,
    }
}
