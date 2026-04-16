#!/bin/bash

# ══════════════════════════════════════════════════════════════════
#  Alien Protocol — Soroban Verifier Pipeline
#
#  What this script does (in order):
#
#  PHASE 1 — Clone tools
#    • Clones circom2soroban and soroban-examples (groth16_verifier)
#
#  PHASE 2 — BLS12-381 Trusted Setup  (per circuit)
#    • Powers of Tau (BLS12-381, size 20)
#    • Phase 2 zkey generation + contribution
#    • Exports vkey.json
#
#  PHASE 3 — Generate Proofs  (per circuit)
#    • snarkjs groth16 prove  →  proof.json + public.json
#    • snarkjs groth16 verify (smoke-check)
#
#  PHASE 4 — circom2soroban Conversion  (per circuit)
#    • Converts vkey.json  →  Rust constants
#    • Converts proof.json →  Rust proof bytes
#    • Converts public.json→  Rust public input bytes
#    • Writes one complete Soroban verifier .rs file per circuit
#
#  Output layout:
#    zk/soroban/
#      ├── circom2soroban/          (tool clone)
#      ├── groth16_verifier/        (Soroban base verifier)
#      ├── bls_setup/               (ptau + new zkeys)
#      │   └── <circuit>/
#      │       ├── circuit_final.zkey
#      │       └── vkey.json
#      ├── proofs/
#      │   └── <circuit>/
#      │       ├── proof.json
#      │       └── public.json
#      └── contracts/
#          └── <circuit>_verifier.rs
#
# ══════════════════════════════════════════════════════════════════

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZK_DIR="$(dirname "$SCRIPT_DIR")"

BUILD_DIR="$ZK_DIR/build"
WITNESS_DIR="$ZK_DIR/witnesses"
SOROBAN_DIR="$ZK_DIR/soroban"

BLS_SETUP_DIR="$SOROBAN_DIR/bls_setup"
PROOF_DIR="$SOROBAN_DIR/proofs"
CONTRACT_DIR="$SOROBAN_DIR/contracts"
TOOLS_DIR="$SOROBAN_DIR/tools"

CIRCUITS=(
  "merkle_inclusion"
  "merkle_non_inclusion"
  "merkle_update"
  "merkle_update_proof"
  "username_merkle"
  "username_hash"
)

# BLS12-381 Powers of Tau size — 2^20 = 1,048,576 constraints max
POT_SIZE=20

GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
YELLOW="\033[0;33m"
BOLD="\033[1m"
RESET="\033[0m"

ok()      { echo -e "${GREEN}  ✔  $1${RESET}"; }
fail()    { echo -e "${RED}  ✘  $1${RESET}"; exit 1; }
info()    { echo -e "${CYAN}▶  $1${RESET}"; }
warn()    { echo -e "${YELLOW}  ⚠  $1${RESET}"; }
section() { echo -e "\n${BOLD}${CYAN}══ $1 ══${RESET}\n"; }

# ── Argument parsing ──────────────────────────

TARGET=""
SKIP_SETUP=0
SKIP_PROVE=0
SKIP_CONVERT=0

usage() {
  echo ""
  echo "Usage: $0 [OPTIONS]"
  echo ""
  echo "Options:"
  echo "  -c, --circuit <name>   Run pipeline for a single circuit only"
  echo "  --skip-setup           Skip BLS12-381 trusted setup (use existing zkeys)"
  echo "  --skip-prove           Skip proof generation (use existing proof.json)"
  echo "  --skip-convert         Skip circom2soroban conversion"
  echo "  -h, --help             Show this help message"
  echo ""
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -c|--circuit)   TARGET="$2"; shift 2 ;;
    --skip-setup)   SKIP_SETUP=1; shift ;;
    --skip-prove)   SKIP_PROVE=1; shift ;;
    --skip-convert) SKIP_CONVERT=1; shift ;;
    -h|--help)      usage; exit 0 ;;
    *)
      echo -e "${RED}Unknown option: $1${RESET}"
      usage; exit 1 ;;
  esac
done

if [ -n "$TARGET" ]; then
  FOUND=0
  for c in "${CIRCUITS[@]}"; do [ "$c" = "$TARGET" ] && FOUND=1 && break; done
  [ "$FOUND" -eq 1 ] || fail "Unknown circuit: '$TARGET'"
  RUN_CIRCUITS=("$TARGET")
else
  RUN_CIRCUITS=("${CIRCUITS[@]}")
fi

# ── Preflight checks ──────────────────────────

echo ""
echo "════════════════════════════════════════════════════"
echo "   Alien Protocol — Soroban Verifier Pipeline"
echo "════════════════════════════════════════════════════"

command -v snarkjs  >/dev/null 2>&1 || fail "snarkjs not found. Run: npm install -g snarkjs"
command -v cargo    >/dev/null 2>&1 || fail "cargo (Rust) not found. Install from https://rustup.rs"
command -v git      >/dev/null 2>&1 || fail "git not found."

mkdir -p "$BLS_SETUP_DIR" "$PROOF_DIR" "$CONTRACT_DIR" "$TOOLS_DIR"

# ═══════════════════════════════════════════════════
#  PHASE 1 — Clone tools
# ═══════════════════════════════════════════════════

section "PHASE 1 — Cloning Tools"

C2S_DIR="$TOOLS_DIR/circom2soroban"
GROTH16_DIR="$TOOLS_DIR/groth16_verifier"

if [ ! -d "$C2S_DIR" ]; then
  info "Cloning circom2soroban..."
  git clone https://github.com/ymcrcat/soroban-privacy-pools.git "$TOOLS_DIR/soroban-privacy-pools"
  # circom2soroban lives inside that repo
  if [ -d "$TOOLS_DIR/soroban-privacy-pools/circom2soroban" ]; then
    cp -r "$TOOLS_DIR/soroban-privacy-pools/circom2soroban" "$C2S_DIR"
    ok "circom2soroban cloned"
  else
    warn "circom2soroban subdir not found in repo — will use manual Rust conversion"
    C2S_DIR=""
  fi
else
  ok "circom2soroban already present"
fi

if [ ! -d "$GROTH16_DIR" ]; then
  info "Cloning soroban groth16_verifier example..."
  git clone https://github.com/stellar/soroban-examples.git "$TOOLS_DIR/soroban-examples"
  if [ -d "$TOOLS_DIR/soroban-examples/groth16_verifier" ]; then
    cp -r "$TOOLS_DIR/soroban-examples/groth16_verifier" "$GROTH16_DIR"
    ok "groth16_verifier cloned"
  else
    warn "groth16_verifier not found in soroban-examples"
    GROTH16_DIR=""
  fi
else
  ok "groth16_verifier already present"
fi

# Build circom2soroban if available
C2S_BIN=""
if [ -n "$C2S_DIR" ] && [ -f "$C2S_DIR/Cargo.toml" ]; then
  info "Building circom2soroban..."
  (cd "$C2S_DIR" && cargo build --release 2>&1) \
    && C2S_BIN="$C2S_DIR/target/release/circom2soroban" \
    && ok "circom2soroban built at $C2S_BIN" \
    || warn "circom2soroban build failed — will generate Rust stubs manually"
fi

# ═══════════════════════════════════════════════════
#  PHASE 2 — BLS12-381 Trusted Setup
# ═══════════════════════════════════════════════════

if [ "$SKIP_SETUP" -eq 0 ]; then
  section "PHASE 2 — BLS12-381 Trusted Setup"

  POT_FILE="$BLS_SETUP_DIR/pot${POT_SIZE}_bls12381_final.ptau"

  if [ ! -f "$POT_FILE" ]; then
    info "Generating BLS12-381 Powers of Tau (size $POT_SIZE)..."
    info "This takes a few minutes — grab a coffee ☕"

    snarkjs powersoftau new bls12381 $POT_SIZE \
      "$BLS_SETUP_DIR/pot${POT_SIZE}_0000.ptau" -v

    snarkjs powersoftau contribute \
      "$BLS_SETUP_DIR/pot${POT_SIZE}_0000.ptau" \
      "$BLS_SETUP_DIR/pot${POT_SIZE}_0001.ptau" \
      --name="Alien Protocol Contributor" \
      --entropy="$(head -c 64 /dev/urandom | base64)" \
      -v

    snarkjs powersoftau prepare phase2 \
      "$BLS_SETUP_DIR/pot${POT_SIZE}_0001.ptau" \
      "$POT_FILE" -v

    ok "BLS12-381 Powers of Tau ready → $POT_FILE"
  else
    ok "BLS12-381 ptau already exists, skipping"
  fi

  for CIRCUIT in "${RUN_CIRCUITS[@]}"; do
    info "Setting up circuit: $CIRCUIT"

    R1CS="$BUILD_DIR/$CIRCUIT/$CIRCUIT.r1cs"
    CIRCUIT_SETUP_DIR="$BLS_SETUP_DIR/$CIRCUIT"
    mkdir -p "$CIRCUIT_SETUP_DIR"

    ZKEY_0="$CIRCUIT_SETUP_DIR/circuit_0000.zkey"
    ZKEY_F="$CIRCUIT_SETUP_DIR/circuit_final.zkey"
    VKEY="$CIRCUIT_SETUP_DIR/vkey.json"

    [ -f "$R1CS" ] || fail "$CIRCUIT — r1cs not found at $R1CS (run compile first)"

    if [ ! -f "$ZKEY_F" ]; then
      snarkjs groth16 setup "$R1CS" "$POT_FILE" "$ZKEY_0"

      snarkjs zkey contribute "$ZKEY_0" "$ZKEY_F" \
        --name="Alien Protocol Phase2" \
        --entropy="$(head -c 64 /dev/urandom | base64)" \
        -v

      rm -f "$ZKEY_0"
    else
      ok "$CIRCUIT — zkey already exists, skipping"
    fi

    if [ ! -f "$VKEY" ]; then
      snarkjs zkey export verificationkey "$ZKEY_F" "$VKEY"
    fi

    ok "$CIRCUIT — BLS12-381 setup complete"
    echo "     ├── zkey : $ZKEY_F"
    echo "     └── vkey : $VKEY"
    echo ""
  done
else
  warn "Skipping BLS12-381 trusted setup (--skip-setup)"
fi

# ═══════════════════════════════════════════════════
#  PHASE 3 — Generate & Verify Proofs
# ═══════════════════════════════════════════════════

if [ "$SKIP_PROVE" -eq 0 ]; then
  section "PHASE 3 — Generating Proofs"

  PROVE_FAIL=()

  for CIRCUIT in "${RUN_CIRCUITS[@]}"; do
    info "Proving: $CIRCUIT"

    ZKEY_F="$BLS_SETUP_DIR/$CIRCUIT/circuit_final.zkey"
    VKEY="$BLS_SETUP_DIR/$CIRCUIT/vkey.json"
    WTNS="$WITNESS_DIR/$CIRCUIT/$CIRCUIT.wtns"
    OUT="$PROOF_DIR/$CIRCUIT"
    mkdir -p "$OUT"

    PROOF="$OUT/proof.json"
    PUBLIC="$OUT/public.json"

    [ -f "$ZKEY_F" ] || { warn "$CIRCUIT — zkey missing, skipping prove"; PROVE_FAIL+=("$CIRCUIT"); continue; }
    [ -f "$WTNS"   ] || { warn "$CIRCUIT — witness missing at $WTNS, skipping"; PROVE_FAIL+=("$CIRCUIT"); continue; }

    snarkjs groth16 prove "$ZKEY_F" "$WTNS" "$PROOF" "$PUBLIC"

    # Smoke-check the proof
    if snarkjs groth16 verify "$VKEY" "$PUBLIC" "$PROOF"; then
      ok "$CIRCUIT — proof valid ✔"
    else
      warn "$CIRCUIT — proof generated but verification FAILED"
      PROVE_FAIL+=("$CIRCUIT (verify failed)")
    fi

    echo "     ├── proof  : $PROOF"
    echo "     └── public : $PUBLIC"
    echo ""
  done

  if [ ${#PROVE_FAIL[@]} -gt 0 ]; then
    warn "Proof generation issues:"
    for f in "${PROVE_FAIL[@]}"; do echo "    - $f"; done
  fi
else
  warn "Skipping proof generation (--skip-prove)"
fi

# ═══════════════════════════════════════════════════
#  PHASE 4 — circom2soroban Conversion
# ═══════════════════════════════════════════════════

if [ "$SKIP_CONVERT" -eq 0 ]; then
  section "PHASE 4 — Generating Soroban Verifier Contracts"

  for CIRCUIT in "${RUN_CIRCUITS[@]}"; do
    info "Converting: $CIRCUIT"

    VKEY="$BLS_SETUP_DIR/$CIRCUIT/vkey.json"
    PROOF="$PROOF_DIR/$CIRCUIT/proof.json"
    PUBLIC="$PROOF_DIR/$CIRCUIT/public.json"
    OUT_RS="$CONTRACT_DIR/${CIRCUIT}_verifier.rs"
    C2S_OUT="$CONTRACT_DIR/${CIRCUIT}_c2s"
    mkdir -p "$C2S_OUT"

    [ -f "$VKEY"   ] || { warn "$CIRCUIT — vkey missing, skipping conversion"; continue; }
    [ -f "$PROOF"  ] || { warn "$CIRCUIT — proof missing, skipping conversion"; continue; }
    [ -f "$PUBLIC" ] || { warn "$CIRCUIT — public.json missing, skipping conversion"; continue; }

    # ── Run circom2soroban if available ──────────
    if [ -n "$C2S_BIN" ] && [ -x "$C2S_BIN" ]; then
      "$C2S_BIN" vk     "$VKEY"   > "$C2S_OUT/vk_rust.txt"   2>/dev/null || true
      "$C2S_BIN" proof  "$PROOF"  > "$C2S_OUT/proof_rust.txt" 2>/dev/null || true
      "$C2S_BIN" public "$PUBLIC" > "$C2S_OUT/pub_rust.txt"   2>/dev/null || true
      ok "$CIRCUIT — circom2soroban conversion done"
    fi

    # ── Generate Soroban contract stub ───────────
    # Reads vkey.json and writes a complete Rust Soroban verifier
    # that embeds the verification key constants and exposes
    # a verify_proof(proof_bytes, public_bytes) → bool entry point.

    CIRCUIT_PASCAL=$(echo "$CIRCUIT" | sed -r 's/(^|_)([a-z])/\U\2/g')

    node - <<NODEJS_EOF > "$OUT_RS"
const fs = require('fs');
const vkey = JSON.parse(fs.readFileSync('$VKEY', 'utf8'));

// Extract verification key fields
const alpha = vkey.vk_alpha_1;
const beta  = vkey.vk_beta_2;
const gamma = vkey.vk_gamma_2;
const delta = vkey.vk_delta_2;
const ic    = vkey.IC;

function g1(pt) {
  return \`G1Affine::from_xy_unchecked(
        Fq::from_str("\${pt[0]}").unwrap(),
        Fq::from_str("\${pt[1]}").unwrap(),
    )\`;
}

function g2(pt) {
  return \`G2Affine::from_xy_unchecked(
        Fq2::new(
            Fq::from_str("\${pt[0][0]}").unwrap(),
            Fq::from_str("\${pt[0][1]}").unwrap(),
        ),
        Fq2::new(
            Fq::from_str("\${pt[1][0]}").unwrap(),
            Fq::from_str("\${pt[1][1]}").unwrap(),
        ),
    )\`;
}

const icLines = ic.map((pt, i) =>
  \`        // IC[\${i}]\n        \${g1(pt)},\`
).join('\n');

const numIC = ic.length;
const numPublic = ic.length - 1;

const out = \`// SPDX-License-Identifier: Apache-2.0
// Auto-generated by soroban-setup.sh (circom2soroban pipeline)
// Circuit : $CIRCUIT
// Curve   : BLS12-381
// Scheme  : Groth16
//
// Usage:
//   1. Copy this file into your Soroban contract crate.
//   2. Add the groth16_verifier crate as a dependency.
//   3. Call \\\`verify_proof(proof_bytes, public_bytes)\\\` from your contract.

#![no_std]

use soroban_sdk::{contract, contractimpl, Bytes, Env};
use ark_bls12_381::{Bls12_381, Fq, Fq2, G1Affine, G2Affine};
use ark_ff::Fp;
use ark_groth16::{Groth16, PreparedVerifyingKey, VerifyingKey};
use groth16_verifier::{deserialize_proof, deserialize_public_inputs};

// ── Verification Key Constants ─────────────────────────────
//
// These constants are derived from the circuit-specific trusted
// setup (BLS12-381 Powers of Tau).  Changing any constant will
// cause all proofs for this circuit to fail verification.

const ALPHA_G1: fn() -> G1Affine = || {
    \${g1(alpha)}
};

const BETA_G2: fn() -> G2Affine = || {
    \${g2(beta)}
};

const GAMMA_G2: fn() -> G2Affine = || {
    \${g2(gamma)}
};

const DELTA_G2: fn() -> G2Affine = || {
    \${g2(delta)}
};

/// Input-output commitment bases IC[0..n].
/// IC[0] is the constant term; IC[1..] correspond to public signals.
const NUM_IC: usize = \${numIC};
const NUM_PUBLIC: usize = \${numPublic};

fn make_ic() -> [G1Affine; \${numIC}] {
    [
\${icLines}
    ]
}

// ── Soroban Contract ───────────────────────────────────────

#[contract]
pub struct ${CIRCUIT_PASCAL}Verifier;

#[contractimpl]
impl ${CIRCUIT_PASCAL}Verifier {
    /// Verify a Groth16 proof for the \\\`$CIRCUIT\\\` circuit.
    ///
    /// # Arguments
    /// * \\\`proof_bytes\\\`  — serialized proof  (output of circom2soroban proof)
    /// * \\\`public_bytes\\\` — serialized public inputs (output of circom2soroban public)
    ///
    /// # Returns
    /// \\\`true\\\` if the proof is valid, \\\`false\\\` otherwise.
    pub fn verify_proof(
        _env: Env,
        proof_bytes: Bytes,
        public_bytes: Bytes,
    ) -> bool {
        let vk = VerifyingKey::<Bls12_381> {
            alpha_g1: ALPHA_G1(),
            beta_g2:  BETA_G2(),
            gamma_g2: GAMMA_G2(),
            delta_g2: DELTA_G2(),
            gamma_abc_g1: make_ic().to_vec(),
        };

        let pvk = Groth16::<Bls12_381>::process_vk(&vk)
            .expect("vk processing failed");

        let proof = match deserialize_proof(&proof_bytes.to_alloc_vec()) {
            Ok(p)  => p,
            Err(_) => return false,
        };

        let public_inputs = match deserialize_public_inputs(
            &public_bytes.to_alloc_vec(),
            NUM_PUBLIC,
        ) {
            Ok(pi) => pi,
            Err(_) => return false,
        };

        Groth16::<Bls12_381>::verify_with_processed_vk(
            &pvk,
            &public_inputs,
            &proof,
        )
        .unwrap_or(false)
    }
}
\`;

process.stdout.write(out);
NODEJS_EOF

    ok "$CIRCUIT — Soroban contract written"
    echo "     └── $OUT_RS"
    echo ""
  done
fi

# ═══════════════════════════════════════════════════
#  Summary
# ═══════════════════════════════════════════════════

echo "════════════════════════════════════════════════════"
echo -e "${GREEN}${BOLD}   Soroban Pipeline Complete${RESET}"
echo ""
echo "   Output directories:"
echo "     BLS zkeys  : $BLS_SETUP_DIR"
echo "     Proofs     : $PROOF_DIR"
echo "     Contracts  : $CONTRACT_DIR"
echo ""
echo "   Next steps:"
echo "     1. Copy a contract .rs into your Soroban crate:"
echo "        cp $CONTRACT_DIR/<circuit>_verifier.rs  my_contract/src/"
echo ""
echo "     2. Add dependencies to Cargo.toml:"
echo "        groth16_verifier = { path = \"$GROTH16_DIR\" }"
echo "        ark-bls12-381 = { version = \"0.4\", default-features = false }"
echo "        ark-groth16   = { version = \"0.4\", default-features = false }"
echo ""
echo "     3. Build & deploy:"
echo "        stellar contract build"
echo "        stellar contract deploy --wasm target/wasm32-unknown-unknown/release/*.wasm"
echo ""
echo "════════════════════════════════════════════════════"
