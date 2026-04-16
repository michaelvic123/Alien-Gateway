const { buildPoseidon } = require("circomlibjs");
const { writeFileSync } = require("fs");
const path = require("path");

const LEVELS = 20;
const OUTPUT_PATH = path.join(__dirname, "..", "inputs", "merkle_non_inclusion.json");

function buildEmptyHashes(poseidon, depth) {
  const F = poseidon.F;
  const hashes = [0n];
  for (let i = 0; i < depth; i++) {
    hashes.push(F.toObject(poseidon([hashes[i], hashes[i]])));
  }
  return hashes;
}

function computeRoot(poseidon, leaf, siblings, indices) {
  const F = poseidon.F;
  let current = leaf;
  for (let i = 0; i < siblings.length; i++) {
    const [left, right] = indices[i] === 0 ? [current, siblings[i]] : [siblings[i], current];
    current = F.toObject(poseidon([left, right]));
  }
  return current;
}

function usernameHash(poseidon, username) {
  const F = poseidon.F;
  const chunks = [];

  for (let i = 0; i < 8; i++) {
    chunks[i] = F.toObject(
      poseidon([
        username[i * 4 + 0],
        username[i * 4 + 1],
        username[i * 4 + 2],
        username[i * 4 + 3],
      ])
    );
  }

  const merged = [];
  for (let i = 0; i < 2; i++) {
    const offset = i * 4;
    merged[i] = F.toObject(
      poseidon([
        chunks[offset + 0],
        chunks[offset + 1],
        chunks[offset + 2],
        chunks[offset + 3],
      ])
    );
  }

  return F.toObject(poseidon([merged[0], merged[1]]));
}

async function main() {
  const poseidon = await buildPoseidon();
  const emptyHashes = buildEmptyHashes(poseidon, LEVELS);
  const limit = 1n << 252n;

  let username = null;
  let hash = null;
  let candidate = null;

  // The circuit range-checks values with Num2Bits(252), so the hash and
  // boundary leaves must stay strictly below 2^252.
  for (let i = 1n; i < 5000n; i++) {
    const nextUsername = new Array(32).fill(0n);
    nextUsername[0] = i;

    const nextHash = usernameHash(poseidon, nextUsername);
    if (nextHash > 1n && nextHash < limit - 2n) {
      candidate = i;
      username = nextUsername;
      hash = nextHash;
      break;
    }
  }

  if (username === null || hash === null || candidate === null) {
    throw new Error("failed to find a username hash below 2^252");
  }

  const leafBefore = hash - 1n;
  const leafAfter = hash + 1n;

  const merklePathBeforeIndices = new Array(LEVELS).fill(0);
  const merklePathAfterIndices = new Array(LEVELS).fill(0);
  merklePathAfterIndices[0] = 1;

  const merklePathBeforeSiblings = emptyHashes.slice(0, LEVELS);
  const merklePathAfterSiblings = emptyHashes.slice(0, LEVELS);
  merklePathBeforeSiblings[0] = leafAfter;
  merklePathAfterSiblings[0] = leafBefore;

  const root = computeRoot(
    poseidon,
    leafBefore,
    merklePathBeforeSiblings,
    merklePathBeforeIndices
  );

  const input = {
    username: username.map((value) => value.toString()),
    leaf_before: leafBefore.toString(),
    leaf_after: leafAfter.toString(),
    merklePathBeforeSiblings: merklePathBeforeSiblings.map((value) => value.toString()),
    merklePathBeforeIndices,
    merklePathAfterSiblings: merklePathAfterSiblings.map((value) => value.toString()),
    merklePathAfterIndices,
    root: root.toString(),
  };

  writeFileSync(OUTPUT_PATH, JSON.stringify(input, null, 2) + "\n");

  console.log(`candidate = ${candidate.toString()}`);
  console.log(`hash = ${hash.toString()}`);
  console.log(`wrote ${OUTPUT_PATH}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
