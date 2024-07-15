use katana_primitives::transaction::InvokeTx;
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct ProgramInputV2 {
    pub block_number: u64,
    pub invokes: Vec<InvokeTx>,
}

pub fn serialize_to_prover_args(input: Vec<InvokeTx>) -> Vec<FieldElement> {
    let mut out: Vec<FieldElement> = vec![];

    for invoke in input {
        let calldata = match invoke {
            InvokeTx::V1(tx) => tx.calldata,
            InvokeTx::V3(tx) => tx.calldata,
        };

        out.push(FieldElement::from(0_usize));
        out.push(FieldElement::from(0_usize));
        out.push(FieldElement::from(calldata.len()));
        out.extend(calldata.iter().cloned());
    }
    
    out
}

impl ProgramInputV2 {
    pub fn prepare_differ_args(inputs: Vec<ProgramInputV2>) -> String {
        let serialized =
            inputs.iter().flat_map(|input| serialize_to_prover_args(input.invokes.clone())).collect::<Vec<_>>();

        let joined = serialized.iter().map(|f| f.to_big_decimal(0).to_string()).collect::<Vec<_>>();

        format!("[{} {}]", inputs.len(), joined.join(" "))
    }
}