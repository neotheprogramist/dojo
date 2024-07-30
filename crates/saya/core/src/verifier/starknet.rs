use anyhow::Context;
use dojo_world::migration::TxnConfig;
use dojo_world::utils::TransactionExt;
use itertools::Itertools;
use starknet::accounts::{Account, Call, ConnectedAccount};
use starknet::core::types::FieldElement;
use starknet::core::utils::get_selector_from_name;
use starknet_crypto::poseidon_hash_many;
use tracing::{trace, warn};

use crate::dojo_os::STARKNET_ACCOUNT;
use crate::verifier::utils::wait_for_sent_transaction;

pub async fn starknet_verify(
    fact_registry_address: FieldElement,
    serialized_proof: Vec<FieldElement>,
    cairo_version: FieldElement,
) -> anyhow::Result<(String, FieldElement)> {
    if serialized_proof.len() > 2000 {
        warn!(
            "Calldata too long at: {} felts, transaction could fail, splitting it.",
            serialized_proof.len()
        );
    }
    let txn_config = TxnConfig { wait: true, receipt: true, ..Default::default() };

    let mut nonce = STARKNET_ACCOUNT.get_nonce().await?;
    let mut hashes = Vec::new();

    for fragment in serialized_proof.into_iter().chunks(2000).into_iter() {
        let mut fragment = fragment.collect::<Vec<_>>();
        let hash = poseidon_hash_many(&fragment);
        hashes.push(hash);

        fragment.insert(0, fragment.len().into());

        let tx = STARKNET_ACCOUNT
            .execute(vec![Call {
                to: fact_registry_address,
                selector: get_selector_from_name("publish_fragment").expect("invalid selector"),
                calldata: fragment,
            }])
            .nonce(nonce)
            // .max_fee(576834050002014927u64.into())
            .send_with_cfg(&txn_config)
            .await
            .context("Failed to send `publish_fragment` transaction.")?;

        trace!("Sent `publish_fragment` transaction {:#x}", tx.transaction_hash);

        wait_for_sent_transaction(tx).await?;

        nonce += 1u64.into();
    }

    let nonce = STARKNET_ACCOUNT.get_nonce().await?;

    let calldata = [FieldElement::from(hashes.len() as u64)]
        .into_iter()
        .chain(hashes.into_iter())
        .chain([cairo_version].into_iter())
        .collect::<Vec<_>>();

    let tx = STARKNET_ACCOUNT
        .execute(vec![Call {
            to: fact_registry_address,
            selector: get_selector_from_name("verify_and_register_fact_from_fragments")
                .expect("invalid selector"),
            calldata,
        }])
        .nonce(nonce)
        .send_with_cfg(&txn_config)
        .await
        .context("Failed to send `verify_and_register_fact_from_fragments` transaction.")?;

    let transaction_hash = format!("{:#x}", tx.transaction_hash);
    wait_for_sent_transaction(tx).await?;

    Ok((transaction_hash, nonce + 1u64.into()))
}
