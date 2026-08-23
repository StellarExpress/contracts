#!/usr/bin/env bash
# Builds and deploys the StellarExpress escrow contract to a Stellar network.
#
# Usage: ./scripts/deploy.sh <network> <source-account>
#   network:        local | testnet | futurenet | mainnet (default: testnet)
#   source-account:  a funded account/key alias known to `stellar keys`
set -euo pipefail

NETWORK="${1:-testnet}"
SOURCE="${2:-escrow-deployer}"

echo "==> Building escrow contract (wasm32v1-none, release, optimized)"
stellar contract build

WASM_PATH="target/wasm32v1-none/release/escrow.wasm"

echo "==> Deploying to $NETWORK as $SOURCE"
CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_PATH" \
  --source "$SOURCE" \
  --network "$NETWORK")

echo "==> Deployed escrow contract: $CONTRACT_ID"
echo "$CONTRACT_ID" > ".contract-id.$NETWORK"
echo "Saved to .contract-id.$NETWORK"

echo "==> Don't forget to call init(admin, arbiter) once before first use:"
echo "    stellar contract invoke --id $CONTRACT_ID --source $SOURCE --network $NETWORK -- init --admin <ADMIN_G...> --arbiter <ARBITER_G...>"
