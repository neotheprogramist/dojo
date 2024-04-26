use super::{prove, ProgramInput, ProverIdentifier};
use futures::future::BoxFuture;
use futures::FutureExt;
use tracing::{info, trace};

async fn combine_proofs(
    first: Vec<String>,
    second: Vec<String>,
    _input: &ProgramInput,
) -> anyhow::Result<Vec<String>> {
    return Ok(first.into_iter().chain(second.into_iter()).collect());
}

/// Simulates the proving process with a placeholder function.
/// Returns a proof string asynchronously.
/// Handles the recursive proving of blocks using asynchronous futures.
/// It returns a BoxFuture to allow for dynamic dispatch of futures, useful in recursive async
/// calls.
pub fn prove_recursively(
    mut inputs: Vec<ProgramInput>,
    prover: ProverIdentifier,
) -> BoxFuture<'static, anyhow::Result<(Vec<String>, ProgramInput)>> {
    async move {
        if inputs.len() == 1 {
            let input = inputs.pop().unwrap();
            let block_number = input.block_number;
            trace!(target: "saya_core", "Proving block {block_number}");
            let proof = prove(serde_json::to_string(&input)?, ProverIdentifier::Stone).await?;
            info!(target: "saya_core", block_number, "Block proven");
            let result = vec![proof];
            Ok((result, input))
        } else {
            let mid = inputs.len() / 2;
            let last = inputs.split_off(mid);

            let (earlier, later) = tokio::try_join!(
                tokio::spawn(async move { prove_recursively(inputs, prover.clone()).await }),
                tokio::spawn(async move { prove_recursively(last, prover).await })
            )?;
            let (earlier, later) = (earlier?, later?);

            let input = earlier.1.combine(later.1);
            let merged_proofs = combine_proofs(earlier.0, later.0, &input).await?;
            Ok((merged_proofs, input))
        }
    }
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use katana_primitives::FieldElement;

    #[tokio::test]
    async fn test_one() {
        let inputs = (0..1)
            .map(|i| ProgramInput {
                prev_state_root: FieldElement::from(i),
                block_number: i,
                block_hash: FieldElement::from(i),
                config_hash: FieldElement::from(i),
                message_to_appchain_segment: Default::default(),
                message_to_starknet_segment: Default::default(),
                state_updates: Default::default(),
            })
            .collect::<Vec<_>>();

        let proof = prove_recursively(inputs.clone(), ProverIdentifier::Stone)
            .await
            .unwrap()
            .0
            .pop()
            .unwrap();
        let expected =
            prove(serde_json::to_string(&inputs).unwrap(), ProverIdentifier::Stone).await.unwrap();
        assert_eq!(proof, expected);
    }

    #[tokio::test]
    async fn test_combined() {
        let inputs = (0..2)
            .map(|i| ProgramInput {
                prev_state_root: FieldElement::from(i),
                block_number: i,
                block_hash: FieldElement::from(i),
                config_hash: FieldElement::from(i),
                message_to_appchain_segment: Default::default(),
                message_to_starknet_segment: Default::default(),
                state_updates: Default::default(),
            })
            .collect::<Vec<_>>();
        let cloned_inputs = inputs.clone();
        let proof = prove_recursively(cloned_inputs, ProverIdentifier::Stone).await.unwrap();
        let expected1 = prove(serde_json::to_string(&inputs[0]).unwrap(), ProverIdentifier::Stone)
            .await
            .unwrap();
        let expected2 = prove(serde_json::to_string(&inputs[1]).unwrap(), ProverIdentifier::Stone)
            .await
            .unwrap();
        assert_eq!(proof.0[0], expected1);
        assert_eq!(proof.0[1], expected2);
    }

    #[tokio::test]
    async fn test_many() {
        let inputs = (0..4)
            .map(|i| ProgramInput {
                prev_state_root: FieldElement::from(i),
                block_number: i,
                block_hash: FieldElement::from(i),
                config_hash: FieldElement::from(i),
                message_to_appchain_segment: Default::default(),
                message_to_starknet_segment: Default::default(),
                state_updates: Default::default(),
            })
            .collect::<Vec<_>>();

        let cloned_inputs = inputs.clone();
        let proof = prove_recursively(cloned_inputs, ProverIdentifier::Stone).await.unwrap();
        let expected1 =
            prove(serde_json::to_string(&inputs).unwrap(), ProverIdentifier::Stone).await.unwrap();
        let expected2 =
            prove(serde_json::to_string(&inputs).unwrap(), ProverIdentifier::Stone).await.unwrap();
        let expected3 =
            prove(serde_json::to_string(&inputs).unwrap(), ProverIdentifier::Stone).await.unwrap();
        let expected4 =
            prove(serde_json::to_string(&inputs).unwrap(), ProverIdentifier::Stone).await.unwrap();
        assert_eq!(proof.0[0], expected1);
        assert_eq!(proof.0[1], expected2);
        assert_eq!(proof.0[2], expected3);
        assert_eq!(proof.0[3], expected4);
    }
}
