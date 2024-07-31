#!/bin/bash

# set -a && source .env && set +as

# Set private variables

SAYA_SEPOLIA_ENDPOINT=
SAYA_SEPOLIA_PRIVATE_KEY=
SAYA_SEPOLIA_ACCOUNT_ADDRESS=
SAYA_PROVER_KEY=
SAYA_SNCAST_ACCOUNT_NAME="dev"

# Probably no need to change these

SAYA_PROVER_URL=http://prover.visoft.dev:3618
SAYA_WORLD_NAME=saya-persistent-run
SAYA_MANIFEST_PATH=../shard-dungeon/Scarb.toml
SAYA_FACT_REGISTRY=0x216a9754a38e86a09261ee424012b97d498a0f4ca81653bd4be269d583c7ec9
SAYA_PILTOVER_CLASS_HASH=0x06b71b95e47818934fbbda5ea18fe6838d01012217e5d9825e4d08f42d5349d6
SAYA_PILTOVER_STARTING_STATE_ROOT=0
SAYA_CONFIG_HASH=42
SAYA_PROGRAM_HASH=0x042066b8031c907125abd1acb9265ad2ad4b141858d1e1e3caafb411d9ab71cc

# Set after runnig the script

SAYA_WORLD_ADDRESS=""
SAYA_WORLD_PREPARED="" # Set to anything after preparing the world successfully for the first time
SAYA_FORK_BLOCK_NUMBER=
SAYA_SKIP_MAKING_TRANSACTIONS="" # Set to anything to skip making transactions
SAYA_PILTOVER_ADDRESS=""
SAYA_PILTOVER_PREPARED=""


if [[ -z "${SAYA_WORLD_ADDRESS}" ]]; then
  echo "World address not set: DEPLOYING WORLD"

    # Build world contract
    sozo \
        build \
        --manifest-path $SAYA_MANIFEST_PATH

    sozo \
        migrate apply \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url $SAYA_SEPOLIA_ENDPOINT \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --fee-estimate-multiplier 20 \
        --name $SAYA_WORLD_NAME


    exit 0  

else
  echo "Using world: $SAYA_WORLD_ADDRESS"
fi

if [[ -z "${SAYA_WORLD_PREPARED}" ]]; then
    echo "World not prepared: PREPARING WORLD"
    
    sozo \
        execute $SAYA_WORLD_ADDRESS set_differ_program_hash \
        -c 2265722951651489608338464389196546125983429710081933755514038580032192121109 \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url $SAYA_SEPOLIA_ENDPOINT \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --fee-estimate-multiplier 20 \
        --world $SAYA_WORLD_ADDRESS \
        --wait

    sozo \
        execute $SAYA_WORLD_ADDRESS set_merger_program_hash \
        -c 2265722951651489608338464389196546125983429710081933755514038580032192121109 \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url $SAYA_SEPOLIA_ENDPOINT \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --fee-estimate-multiplier 20 \
        --world $SAYA_WORLD_ADDRESS \
        --wait

    sozo \
        execute $SAYA_WORLD_ADDRESS set_facts_registry \
        -c $SAYA_FACT_REGISTRY \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url $SAYA_SEPOLIA_ENDPOINT \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --fee-estimate-multiplier 20 \
        --world $SAYA_WORLD_ADDRESS \
        --wait

    echo "Set SAYA_WORLD_PREPARED to anything to skip this step next time."

else
  echo "World is already prepared"
fi

if [[ -z "${SAYA_FORK_BLOCK_NUMBER}" ]]; then
    echo "Set SAYA_FORK_BLOCK_NUMBER to the latest block including the preparations (check here https://sepolia.starkscan.co/, remember to switch to sepolia!)."
    echo "You can now run \`cargo run -r --bin katana -- --rpc-url $SAYA_SEPOLIA_ENDPOINT --fork-block-number \$SAYA_FORK_BLOCK_NUMBER\` in another terminal."
    exit 0
fi

if [[ -z "${SAYA_PILTOVER_ADDRESS}" ]]; then
    sncast -a dev -u $SAYA_SEPOLIA_ENDPOINT deploy \
        --class-hash $SAYA_PILTOVER_CLASS_HASH \
        -c $SAYA_SEPOLIA_ACCOUNT_ADDRESS $SAYA_PILTOVER_STARTING_STATE_ROOT $(expr $SAYA_FORK_BLOCK_NUMBER + 1)  0

    echo "Set SAYA_PILTOVER_ADDRESS to the address of the deployed contract."
    exit 0
fi

if [[ -z "${SAYA_PILTOVER_PREPARED}" ]]; then
    sncast -a dev -u $SAYA_SEPOLIA_ENDPOINT --wait invoke \
        --contract-address $SAYA_PILTOVER_ADDRESS --function set_program_info -c $SAYA_PROGRAM_HASH $SAYA_CONFIG_HASH
    sncast -a dev -u $SAYA_SEPOLIA_ENDPOINT --wait invoke \
        --contract-address $SAYA_PILTOVER_ADDRESS --function set_facts_registry -c $SAYA_FACT_REGISTRY
fi


if [[ -z "${SAYA_SKIP_MAKING_TRANSACTIONS}" ]]; then
    cargo run -r --bin sozo -- execute shard_dungeon::systems::metagame::metagame register_player \
        -c str:mateo \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url http://localhost:5050 \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --world $SAYA_WORLD_ADDRESS \
        --wait

    cargo run -r --bin sozo -- execute shard_dungeon::systems::hazard_hall::hazard_hall enter_dungeon \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url http://localhost:5050 \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --world $SAYA_WORLD_ADDRESS \
        --wait

    cargo run -r --bin sozo -- execute shard_dungeon::systems::hazard_hall::hazard_hall fate_strike \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url http://localhost:5050 \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --world $SAYA_WORLD_ADDRESS \
        --wait

    cargo run -r --bin sozo -- execute shard_dungeon::systems::hazard_hall::hazard_hall fate_strike \
        --manifest-path $SAYA_MANIFEST_PATH \
        --rpc-url http://localhost:5050 \
        --private-key $SAYA_SEPOLIA_PRIVATE_KEY \
        --account-address $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
        --world $SAYA_WORLD_ADDRESS \
        --wait
fi


cargo run -r --bin sozo -- model get Inventory $SAYA_SEPOLIA_ACCOUNT_ADDRESS \
    --manifest-path $SAYA_MANIFEST_PATH \
    --rpc-url $SAYA_SEPOLIA_ENDPOINT \
    --world $SAYA_WORLD_ADDRESS

cargo run -r --bin saya -- \
    --mode persistent \
    --rpc-url http://localhost:5050 \
    --registry $SAYA_FACT_REGISTRY \
    --piltover $SAYA_PILTOVER_ADDRESS \
    --world $SAYA_WORLD_ADDRESS \
    --url $SAYA_PROVER_URL \
    --store-proofs \
    --private-key $SAYA_PROVER_KEY \
    --start-block $(expr $SAYA_FORK_BLOCK_NUMBER + 1) 
    # --end-block $(expr $SAYA_FORK_BLOCK_NUMBER + 4)