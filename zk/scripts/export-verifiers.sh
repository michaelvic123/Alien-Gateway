#!/bin/bash

set -e

# ─────────────────────────────────────────────
#  Alien Protocol — Solidity Verifier Exporter
# ─────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZK_DIR="$(dirname "$SCRIPT_DIR")"

BUILD_DIR="$ZK_DIR/build"
CONTRACTS_DIR="$ZK_DIR/verifiers"

# Map: circuit_name -> Solidity contract name
declare -A CONTRACT_NAMES=(
  ["merkle_inclusion"]="MerkleInclusionVerifier"
  ["merkle_non_inclusion"]="MerkleNonInclusionVerifier"
  ["merkle_update"]="MerkleUpdateVerifier"
  ["merkle_update_proof"]="MerkleUpdateProofVerifier"
  ["username_merkle"]="UsernameMerkleVerifier"
  ["username_hash"]="UsernameHashVerifier"
)

CIRCUITS=(
  "merkle_inclusion"
  "merkle_non_inclusion"
  "merkle_update"
  "merkle_update_proof"
  "username_merkle"
  "username_hash"
)

GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
YELLOW="\033[0;33m"
RESET="\033[0m"

ok()   { echo -e "${GREEN}  ✔  $1${RESET}"; }
fail() { echo -e "${RED}  ✘  $1${RESET}"; }
info() { echo -e "${CYAN}▶  $1${RESET}"; }
warn() { echo -e "${YELLOW}  ⚠  $1${RESET}"; }

# ── Argument parsing ──────────────────────────

TARGET=""

usage() {
  echo ""
  echo "Usage: $0 [OPTIONS]"
  echo ""
  echo "Options:"
  echo "  -c, --circuit <name>   Export verifier for a single circuit only"
  echo "  -h, --help             Show this help message"
  echo ""
  echo "Circuits:"
  for c in "${CIRCUITS[@]}"; do
    echo "    - $c  →  ${CONTRACT_NAMES[$c]}.sol"
  done
  echo ""
  echo "Verifier contracts are written to: \$ZK_DIR/contracts/verifiers/"
  echo ""
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -c|--circuit)
      TARGET="$2"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo -e "${RED}Unknown option: $1${RESET}"
      usage; exit 1 ;;
  esac
done

# ── Build circuit list ────────────────────────

if [ -n "$TARGET" ]; then
  FOUND=0
  for c in "${CIRCUITS[@]}"; do
    [ "$c" = "$TARGET" ] && FOUND=1 && break
  done
  if [ "$FOUND" -eq 0 ]; then
    fail "Unknown circuit: '$TARGET'. Run with --help to list valid circuits."
    exit 1
  fi
  RUN_CIRCUITS=("$TARGET")
else
  RUN_CIRCUITS=("${CIRCUITS[@]}")
fi

# ── Preflight ─────────────────────────────────

echo ""
echo "================================================"
echo "   Alien Protocol — Solidity Verifier Exporter"
echo "================================================"
echo ""

mkdir -p "$CONTRACTS_DIR"

# ── Main loop ─────────────────────────────────

PASS=0
FAIL_LIST=()

for CIRCUIT in "${RUN_CIRCUITS[@]}"; do
  CONTRACT="${CONTRACT_NAMES[$CIRCUIT]}"
  ZKEY="$BUILD_DIR/$CIRCUIT/${CIRCUIT}_final.zkey"
  OUT_FILE="$CONTRACTS_DIR/${CONTRACT}.sol"

  info "Exporting: $CIRCUIT  →  ${CONTRACT}.sol"

  # Check zkey exists
  if [ ! -f "$ZKEY" ]; then
    warn "Skipping $CIRCUIT — zkey not found at $ZKEY"
    warn "Run trusted-setup.sh first."
    FAIL_LIST+=("$CIRCUIT (missing zkey)")
    echo ""
    continue
  fi

  # Export Solidity verifier
  if snarkjs zkey export solidityverifier "$ZKEY" "$OUT_FILE"; then
    ok "Exported  →  $OUT_FILE"
  else
    fail "Failed to export $CIRCUIT"
    FAIL_LIST+=("$CIRCUIT (export error)")
    echo ""
    continue
  fi

  # Patch the contract name from the default "Groth16Verifier" to the proper name
  if grep -q "contract Groth16Verifier" "$OUT_FILE"; then
    sed -i "s/contract Groth16Verifier/contract ${CONTRACT}/g" "$OUT_FILE"
    ok "Renamed contract  →  ${CONTRACT}"
  fi

  echo "     └── $OUT_FILE"
  echo ""

  PASS=$((PASS + 1))
done

# ── Summary ───────────────────────────────────

echo "================================================"

if [ ${#FAIL_LIST[@]} -gt 0 ]; then
  echo -e "${RED}   ${#FAIL_LIST[@]} export(s) failed:${RESET}"
  for f in "${FAIL_LIST[@]}"; do
    echo -e "${RED}     - $f${RESET}"
  done
fi

echo -e "${GREEN}   $PASS verifier(s) exported successfully${RESET}"
echo -e "${GREEN}   Output directory: $CONTRACTS_DIR${RESET}"
echo "================================================"
echo ""

[ ${#FAIL_LIST[@]} -eq 0 ] || exit 1
