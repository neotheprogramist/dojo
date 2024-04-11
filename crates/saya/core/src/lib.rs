//! Saya core library.

use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use cairo_proof_parser::output::{extract_output, ExtractOutputResult};
use cairo_proof_parser::program::{extract_program, ExtractProgramResult};
use futures::future::{self, join};
use katana_primitives::block::{BlockNumber, FinalityStatus, SealedBlock, SealedBlockWithStatus};
use katana_primitives::transaction::Tx;
use katana_primitives::FieldElement;
use saya_provider::rpc::JsonRpcProvider;
use saya_provider::Provider as SayaProvider;
use serde::{Deserialize, Serialize};
use starknet_crypto::poseidon_hash_many;
use tracing::{error, info, trace};
use url::Url;

use crate::blockchain::Blockchain;
use crate::data_availability::{DataAvailabilityClient, DataAvailabilityConfig};
use crate::error::SayaResult;
use crate::prover::{extract_messages, parse_proof, prove_stone, ProgramInput};

pub mod blockchain;
pub mod data_availability;
pub mod error;
pub mod prover;
pub mod starknet_os;
pub mod verifier;

pub(crate) const LOG_TARGET: &str = "saya::core";

/// Saya's main configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct SayaConfig {
    #[serde(deserialize_with = "url_deserializer")]
    pub katana_rpc: Url,
    pub start_block: u64,
    pub data_availability: Option<DataAvailabilityConfig>,
    pub world_address: FieldElement,
    pub fact_registry_address: FieldElement,
}

fn url_deserializer<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Url::parse(&s).map_err(serde::de::Error::custom)
}

/// Saya.
pub struct Saya {
    /// The main Saya configuration.
    config: SayaConfig,
    /// The data availability client.
    da_client: Option<Box<dyn DataAvailabilityClient>>,
    /// The provider to fetch dojo from Katana.
    provider: Arc<dyn SayaProvider>,
    /// The blockchain state.
    blockchain: Blockchain,
}

impl Saya {
    /// Initializes a new [`Saya`] instance from the given [`SayaConfig`].
    ///
    /// # Arguments
    ///
    /// * `config` - The main Saya configuration.
    pub async fn new(config: SayaConfig) -> SayaResult<Self> {
        // Currently it's only RPC. But it can be the database
        // file directly in the future or other transports.
        let provider = Arc::new(JsonRpcProvider::new(config.katana_rpc.clone()).await?);

        let da_client = if let Some(da_conf) = &config.data_availability {
            Some(data_availability::client_from_config(da_conf.clone()).await?)
        } else {
            None
        };

        let blockchain = Blockchain::new();

        Ok(Self { config, da_client, provider, blockchain })
    }

    /// Starts the Saya mainloop to fetch and process data.
    ///
    /// Optims:
    /// First naive version to have an overview of all the components
    /// and the process.
    /// Should be refacto in crates as necessary.
    pub async fn start(&mut self) -> SayaResult<()> {
        let poll_interval_secs = 1;
        let mut block = self.config.start_block.max(1); // Genesis block is not proven. We advance to block 1

        let (genesis_block, block_before_the_first) =
            join(self.provider.fetch_block(0), self.provider.fetch_block(block - 1)).await;
        let genesis_state_hash = genesis_block?.header.header.state_root;
        let mut previous_block_state_root = block_before_the_first?.header.header.state_root;

        loop {
            let latest_block = match self.provider.block_number().await {
                Ok(block_number) => block_number,
                Err(e) => {
                    error!(target: LOG_TARGET, error = ?e, "Fetching block.");
                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval_secs)).await;
                    continue;
                }
            };

            if block > latest_block {
                trace!(target: LOG_TARGET, block_number = block, "Waiting for block.");
                tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval_secs)).await;
                continue;
            }

            // Fetch all blocks from the current block to the latest block
            let fetched_blocks = future::try_join_all(
                (block..latest_block).map(|block_number| self.provider.fetch_block(block_number)),
            )
            .await?;

            // shift the state roots to the right by one, as proof of each block is based on the previous state root
            let mut state_roots =
                fetched_blocks.iter().map(|b| b.header.header.state_root).collect::<Vec<_>>();
            state_roots.insert(0, previous_block_state_root);
            previous_block_state_root = state_roots.pop().unwrap();

            let params = fetched_blocks
                .into_iter()
                .zip(state_roots)
                .map(|(b, s)| (b, s, genesis_state_hash))
                .collect::<Vec<_>>();

            for p in params {
                self.process_block(block, p).await?;
                block += 1;
            }
        }
    }

    /// Processes the given block number.
    ///
    /// # Summary
    ///
    /// 1. Pulls state update to update local state accordingly. We may publish DA at this point.
    ///
    /// 2. Pulls all transactions and data required to generate the trace.
    ///
    /// 3. Computes facts for this state transition. We may optimistically register the facts.
    ///
    /// 4. Computes the proof from the trace with a prover.
    ///
    /// 5. Registers the facts + the send the proof to verifier. Not all provers require this step
    ///    (a.k.a. SHARP).
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number.
    async fn process_block(
        &mut self,
        block_number: BlockNumber,
        blocks: (SealedBlock, FieldElement, FieldElement),
    ) -> SayaResult<Option<(ProgramInput, String)>> {
        trace!(target: LOG_TARGET, block_number = %block_number, "Processing block.");

        let (block, prev_state_root, _genesis_state_hash) = blocks;

        let (state_updates, da_state_update) =
            self.provider.fetch_state_updates(block_number).await?;

        if let Some(da) = &self.da_client {
            da.publish_state_diff_felts(&da_state_update).await?;
        }

        let block =
            SealedBlockWithStatus { block: block.clone(), status: FinalityStatus::AcceptedOnL2 };

        let state_updates_to_prove = state_updates.state_updates.clone();
        self.blockchain.update_state_with_block(block.clone(), state_updates)?;

        if block_number == 0 {
            return Ok(None);
        }

        let exec_infos = self.provider.fetch_transactions_executions(block_number).await?;

        if exec_infos.is_empty() {
            trace!(target: "saya_core", block_number, "Skipping empty block.");
            return Ok(None);
        }

        let transactions = block
            .block
            .body
            .iter()
            .filter_map(|t| match &t.transaction {
                Tx::L1Handler(tx) => Some(tx),
                _ => None,
            })
            .collect::<Vec<_>>();

        let (message_to_starknet_segment, message_to_appchain_segment) =
            extract_messages(&exec_infos, transactions);

        let new_program_input = ProgramInput {
            prev_state_root,
            block_number: FieldElement::from(block_number),
            block_hash: block.block.header.hash,
            config_hash: FieldElement::from(0u64),
            message_to_starknet_segment,
            message_to_appchain_segment,
            state_updates: state_updates_to_prove,
        };

        let world_da = new_program_input.da_as_calldata(self.config.world_address);
        let world_da_printable: Vec<String> = world_da.iter().map(|x| x.to_string()).collect();
        trace!(target: LOG_TARGET, "World DA {world_da_printable:?}.");

        trace!(target: LOG_TARGET, "Proving block {block_number}.");
        let proof = prove_stone(new_program_input.serialize(self.config.world_address)?).await?;
        info!(target: LOG_TARGET, block_number, "Block proven.");

        trace!(target: LOG_TARGET, "Verifying block {block_number}.");
        let serialized_proof = parse_proof(&proof).unwrap();
        let transaction_hash =
            verifier::starknet_verify(self.config.fact_registry_address, serialized_proof).await?;
        info!(target: LOG_TARGET, block_number, transaction_hash, "Block verified.");

        let ExtractProgramResult { program: _, program_hash } = extract_program(&proof)?;
        let ExtractOutputResult { program_output: _, program_output_hash } =
            extract_output(&proof)?;
        let expected_fact = poseidon_hash_many(&[program_hash, program_output_hash]).to_string();
        info!(target: LOG_TARGET, expected_fact, "Expected fact.");

        sleep(Duration::from_millis(5000));

        trace!(target: LOG_TARGET, "Applying diffs {block_number}.");
        let ExtractOutputResult { program_output, program_output_hash: _ } =
            extract_output(&proof)?;
        let transaction_hash =
            starknet_os::starknet_apply_diffs(self.config.world_address, world_da, program_output)
                .await?;
        info!(target: LOG_TARGET, block_number, transaction_hash, "Diffs applied.");

        trace!(target: "saya_core", "Verifying block {block_number}.");
        let transaction_hash = verifier::verify(proof.clone(), self.config.verifier).await?; // TODO: If we use scheduler this part is only needed at the end of proving
        info!(target: "saya_core", block_number, transaction_hash, "Block verified.");

        Ok(Some((new_program_input, proof)))
    }
}

impl From<starknet::providers::ProviderError> for error::Error {
    fn from(e: starknet::providers::ProviderError) -> Self {
        Self::KatanaClient(format!("Katana client RPC provider error: {e}"))
    }
}
