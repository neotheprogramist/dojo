use std::env;
use std::path::PathBuf;

use prover_sdk::{Cairo1ProverInput, Cairo1CompiledProgram};
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn load_program() -> anyhow::Result<Cairo1CompiledProgram> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    let program_file = manifest_dir.join("programs/cairo1program.json");
    let mut program_file = File::open(program_file).await?;

    let mut data = String::new();
    program_file.read_to_string(&mut data).await?;
    let program_value: Cairo1CompiledProgram = serde_json::from_str(&data)?;

    Ok(program_value)
}

pub async fn prepare_input_cairo(
    arguments: String,
) -> anyhow::Result<Cairo1ProverInput> {
    let program = load_program().await?;

    let data = vec![arguments];
    let program_input: Value = serde_json::from_str(&serde_json::to_string(&data)?)?;

    Ok(Cairo1ProverInput {
        program,
        program_input,
        layout: "recursive".to_string(),
    })
}
