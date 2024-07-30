use std::time::Duration;

use anyhow::Context;
use dojo_world::migration::TxnConfig;
use dojo_world::utils::TransactionExt;
use itertools::Itertools;
use starknet::accounts::{Account, Call, ConnectedAccount};
use starknet::core::types::{
    FieldElement, InvokeTransactionResult, TransactionExecutionStatus, TransactionStatus,
};
use starknet::core::utils::get_selector_from_name;
use starknet::providers::Provider;
use starknet_crypto::poseidon_hash_many;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::dojo_os::STARKNET_ACCOUNT;

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

        info!("Sent `publish_fragment` transaction {:#x}", tx.transaction_hash);

        wait_for(tx).await?;

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
            calldata: dbg!(calldata),
        }])
        .nonce(nonce)
        .send_with_cfg(&txn_config)
        .await
        .context("Failed to send `verify_and_register_fact_from_fragments` transaction.")?;

    let transaction_hash = format!("{:#x}", tx.transaction_hash);
    wait_for(tx).await?;

    Ok((transaction_hash, nonce + 1u64.into()))
}

async fn wait_for(tx: InvokeTransactionResult) -> anyhow::Result<()> {
    let start_fetching = std::time::Instant::now();
    let wait_for = Duration::from_secs(60);
    let execution_status = loop {
        if start_fetching.elapsed() > wait_for {
            anyhow::bail!("Transaction not mined in {} seconds.", wait_for.as_secs());
        }

        let status =
            match STARKNET_ACCOUNT.provider().get_transaction_status(tx.transaction_hash).await {
                Ok(status) => status,
                Err(_e) => {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

        break match status {
            TransactionStatus::Received => {
                info!("Transaction received.");
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            TransactionStatus::Rejected => {
                anyhow::bail!("Transaction {:#x} rejected.", tx.transaction_hash);
            }
            TransactionStatus::AcceptedOnL2(execution_status) => execution_status,
            TransactionStatus::AcceptedOnL1(execution_status) => execution_status,
        };
    };

    match execution_status {
        TransactionExecutionStatus::Succeeded => {
            info!("Transaction accepted on L2.");
        }
        TransactionExecutionStatus::Reverted => {
            anyhow::bail!("Transaction failed with.");
        }
    }

    Ok(())
}
