use std::sync::Arc;

use anyhow::Context;
use katana_primitives::FieldElement;
use prover_sdk::{ProverSDK, ProverSdkErrors};
use tokio::sync::OnceCell;
use url::Url;

use super::loader::prepare_input_cairo1;
use super::ProveProgram;
use crate::prover::loader::prepare_input_cairo0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpProverParams {
    pub prover_url: Url,
    pub prover_key: prover_sdk::ProverAccessKey,
}

static ONCE: OnceCell<Result<ProverSDK, ProverSdkErrors>> = OnceCell::const_new();

pub async fn http_prove(
    prover_params: Arc<HttpProverParams>,
    input: String,
    prove_program: ProveProgram,
) -> anyhow::Result<String> {
    let prover = ONCE
        .get_or_init(|| async {
            ProverSDK::new(prover_params.prover_key.clone(), prover_params.prover_url.clone()).await
        })
        .await;
    let prover = prover.as_ref().map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if prove_program.cairo_version() == FieldElement::ONE {
        let input = prepare_input_cairo1(input, prove_program).await?;
        prover.prove_cairo1(input).await.context("Failed to prove using the http prover")
    } else {
        let input = prepare_input_cairo0(input, prove_program).await?;
        prover.prove_cairo0(input).await.context("Failed to prove using the http prover")
    }
}
