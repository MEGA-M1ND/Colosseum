// ## Agent session key
//
// Generated in the browser and never sent anywhere. This is the whole point of
// the design: the agent holds this key, the owner's wallet holds the funds, and
// the delegation between them is enforced on-chain.
//
// Persisted to localStorage so a page reload does not orphan a live delegation.
// A real agent would hold this server-side; here it stands in for the agent
// process so the demo runs in one place.

import { Keypair } from "@solana/web3.js";

const STORAGE_KEY = "agent-wallet:session-key";

export function loadSession(): Keypair | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(raw)));
  } catch {
    return null;
  }
}

export function createSession(): Keypair {
  const kp = Keypair.generate();
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(Array.from(kp.secretKey)));
  } catch {
    // Private browsing: the key still works for this page's lifetime.
  }
  return kp;
}

export function clearSession(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* nothing to clear */
  }
}
