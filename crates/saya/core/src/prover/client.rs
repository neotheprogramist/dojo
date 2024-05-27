use std::env;
use std::path::PathBuf;

use anyhow::Context;
use prover_sdk::{Cairo0ProverInput, Cairo1ProverInput, ProverSDK, ProverSdkErrors};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::OnceCell;
use tracing::trace;
use url::Url;

use super::ProveProgram;
use crate::LOG_TARGET;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpProverParams {
    pub prover_url: Url,
    pub prover_key: prover_sdk::ProverAccessKey,
}

static ONCE: OnceCell<Result<ProverSDK, ProverSdkErrors>> = OnceCell::const_new();

async fn load_program(prove_program: ProveProgram) -> anyhow::Result<Value> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let program_file = match (prove_program, cfg!(feature = "cairo1differ")) {
        (_, true) => manifest_dir.join("programs/differ.json"),
        (ProveProgram::Differ, false) => manifest_dir.join("programs/cairo0differ.json"),
        (ProveProgram::Merger, false) => manifest_dir.join("programs/cairo0merger.json"),
    };
    let mut program_file = File::open(program_file).await?;

    let mut data = String::new();
    program_file.read_to_string(&mut data).await?;
    let json_value: Value = serde_json::from_str(&data)?;

    Ok(json_value)
}

pub async fn http_prove(
    prover_params: HttpProverParams,
    input: String,
    prove_program: ProveProgram,
) -> anyhow::Result<String> {
    let prover = ONCE
        .get_or_init(|| async {
            ProverSDK::new(prover_params.prover_key, prover_params.prover_url).await
        })
        .await;
    let prover = prover.as_ref().map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut program = load_program(prove_program).await?;

    let result = if cfg!(feature = "cairo1differ") {
        trace!(target: LOG_TARGET, "Proving with cairo1.");

        if let Value::Object(ref mut obj) = program {
            obj.insert("version".to_string(), Value::Number(serde_json::Number::from(1)));
        }

        let program = serde_json::from_str(&serde_json::to_string(&program)?)?;

        let program_input = Value::Array(vec![Value::String(input)]);
        let input = Cairo1ProverInput { program, program_input };

        prover.prove_cairo1(input).await.context("Failed to prove using the http prover")?
    } else {
        trace!(target: LOG_TARGET, "Proving with cairo0.");

        let program = serde_json::from_str(&serde_json::to_string(&program)?)?;
        let program_input: Value = serde_json::from_str(&input)?;

        let input = Cairo0ProverInput { program, program_input };

        prover.prove_cairo0(input).await.context("Failed to prove using the http prover")?
    };

    Ok(result)
}
