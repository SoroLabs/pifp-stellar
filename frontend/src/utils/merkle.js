/**
 * Verifies a Merkle inclusion proof.
 * @param {string} root - Hex-encoded Merkle root.
 * @param {string} leaf - Hex-encoded leaf hash.
 * @param {number} index - Index of the leaf in the tree.
 * @param {string[]} siblings - Hex-encoded sibling hashes.
 * @returns {Promise<boolean>} - True if the proof is valid.
 */
export async function verifyMerkleProof(root, leaf, index, siblings) {
  let currentHash = hexToBytes(leaf);

  for (const siblingHex of siblings) {
    const sibling = hexToBytes(siblingHex);
    const combined = combineHashes(currentHash, sibling);
    currentHash = await crypto.subtle.digest('SHA-256', combined);
  }

  return bytesToHex(new Uint8Array(currentHash)) === root;
}

function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes) {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

function combineHashes(a, b) {
  const combined = new Uint8Array(a.length + b.length);
  
  // Sort for deterministic hashing (matching the backend's `if left <= right`)
  const aHex = bytesToHex(a);
  const bHex = bytesToHex(b);
  
  if (aHex <= bHex) {
    combined.set(a);
    combined.set(b, a.length);
  } else {
    combined.set(b);
    combined.set(a, b.length);
  }
  
  return combined;
}
