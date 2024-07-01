//! Starknet OS types.
// SNOS is based on blockifier, which is not in sync with
// current primitives.
// And SNOS is for now not used. This work must be resume once
// SNOS is actualized.
// mod felt;
// pub mod input;
// pub mod transaction;

use std::time::Duration;

use anyhow::{bail, Context};
use dojo_world::migration::TxnConfig;
use dojo_world::utils::TransactionExt;
use itertools::chain;
use starknet::accounts::{Account, Call, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount};
use starknet::core::types::{
    BlockId, BlockTag, FieldElement, TransactionExecutionStatus, TransactionStatus,
};
use starknet::core::utils::get_selector_from_name;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use starknet::signers::{LocalWallet, SigningKey};
use tokio::time::sleep;
use url::Url;
// will need to be read from the environment for chains other than sepoia
pub const STARKNET_URL: &str =
    "https://starknet-sepolia.g.alchemy.com/v2/PovJ0plog8O9RxyaPMYAZiKHqZ5LLII_";
pub const CHAIN_ID: &str = "0x00000000000000000000000000000000000000000000534e5f5345504f4c4941";
pub const SIGNER_ADDRESS: &str =
    "0x0566eD97787e5C0ee5F6085Ec22Cea5596B0018EC89739c7c64E7C5BeD7f0319";
pub const SIGNER_KEY: &str = "0x021f655cf5f523e54b80014fb61710362677e6ec07d1415fb6d4bd61faa5147d";

lazy_static::lazy_static!(
    pub static ref STARKNET_ACCOUNT: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet> = {
        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(STARKNET_URL).unwrap(),
        ));

        let signer = FieldElement::from_hex_be(SIGNER_KEY).expect("invalid signer hex");
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(signer));

        let address = FieldElement::from_hex_be(SIGNER_ADDRESS).expect("invalid signer address");
        let chain_id = FieldElement::from_hex_be(CHAIN_ID).expect("invalid chain id");

        let mut account = SingleOwnerAccount::new(provider, signer, address, chain_id, ExecutionEncoding::New);
        account.set_block_id(BlockId::Tag(BlockTag::Pending));
        account
    };
);

pub async fn starknet_apply_diffs(
    world: FieldElement,
    new_state: Vec<FieldElement>,
    program_output: Vec<FieldElement>,
    program_hash: FieldElement,
    nonce: FieldElement,
) -> anyhow::Result<String> {
    let calldata = chain![
        vec![FieldElement::from(new_state.len() as u64 / 2)].into_iter(),
        new_state.clone().into_iter(),
        program_output.into_iter(),
        vec![program_hash],
    ]
    .collect();

    let txn_config = TxnConfig { wait: true, receipt: true, ..Default::default() };
    let tx = STARKNET_ACCOUNT
        .execute(vec![Call {
            to: world,
            selector: get_selector_from_name("upgrade_state").expect("invalid selector"),
            calldata,
        }])
        .nonce(nonce)
        .send_with_cfg(&txn_config)
        .await
        .context("Failed to send `upgrade state` transaction.")?;

    let start_fetching = std::time::Instant::now();
    let wait_for = Duration::from_secs(60);
    let execution_status = loop {
        if start_fetching.elapsed() > wait_for {
            bail!("Transaction not mined in {} seconds.", wait_for.as_secs());
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
                println!("Transaction received.");
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            TransactionStatus::Rejected => {
                bail!("Transaction {:#x} rejected.", tx.transaction_hash);
            }
            TransactionStatus::AcceptedOnL2(execution_status) => execution_status,
            TransactionStatus::AcceptedOnL1(execution_status) => execution_status,
        };
    };

    match execution_status {
        TransactionExecutionStatus::Succeeded => {
            println!("Transaction accepted on L2.");
        }
        TransactionExecutionStatus::Reverted => {
            bail!("Transaction failed with.");
        }
    }

    Ok(format!("{:#x}", tx.transaction_hash))
}
