use katana_primitives::transaction::InvokeTx;
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use starknet_api::core::ContractAddress;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct ProgramInputV2 {
    pub block_number: u64,
    pub invokes: Vec<InvokeTx>,
}

pub fn serialize_to_prover_args(input: Vec<InvokeTx>) -> Vec<FieldElement> {
    let mut out: Vec<FieldElement> = vec![];

    out.push(FieldElement::from(input.len()));
    for invoke in input {
        let (calldata, sender) = match invoke {
            InvokeTx::V1(tx) => (tx.calldata, tx.sender_address),
            InvokeTx::V3(tx) => (tx.calldata, tx.sender_address),
        };

        out.push(FieldElement::from(sender));
        out.push(FieldElement::from(sender));
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

        format!("[{}]", joined.join(" "))
    }
}