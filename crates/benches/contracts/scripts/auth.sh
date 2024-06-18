#!/bin/bash
set -euo pipefail
pushd $(dirname "$0")/..

# export RPC_URL="http://localhost:5050";

export WORLD_ADDRESS=$(cat ./manifests/dev/manifest.json | jq -r '.world.address')

export ACTIONS_ADDRESS=$(cat ./manifests/dev/manifest.json | jq -r '.contracts[] | select(.name == "benches::systems::actions::actions" ).address')

echo "---------------------------------------------------------------------------"
echo world : $WORLD_ADDRESS 
echo " "
echo actions : $ACTIONS_ADDRESS
echo "---------------------------------------------------------------------------"

# enable system -> component authorizations
COMPONENTS=("Position" "Moves" "Alias" "Character" )
AUTH_ARGS=""

for component in ${COMPONENTS[@]}; do
    AUTH_ARGS+=" ${component},${ACTIONS_ADDRESS}"
done

sozo auth grant writer $AUTH_ARGS --world $WORLD_ADDRESS --rpc-url $RPC_URL

sleep 4

echo "Default authorizations have been successfully set."