// ## App
//
// Orchestration lives here; the panels are presentational. The shape of the
// demo is deliberate: the owner sets a budget, the agent works inside it, then
// the agent is attacked and the chain refuses. Everything the panels show comes
// from on-chain state, not from local bookkeeping - the point of the product is
// that the enforcement is not in this code.

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ConnectionProvider,
  WalletProvider,
  useConnection,
  useWallet,
} from "@solana/wallet-adapter-react";
import {
  WalletModalProvider,
  WalletMultiButton,
} from "@solana/wallet-adapter-react-ui";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import "@solana/wallet-adapter-react-ui/styles.css";

import * as aw from "./lib/program";
import { decodeDelegation, type DelegationAccount } from "./lib/accounts";
import { decodeError } from "./lib/errors";
import { clearSession, createSession, loadSession } from "./lib/session";
import { VaultSetup } from "./components/VaultSetup";
import { PolicyEditor } from "./components/PolicyEditor";
import { AgentConsole } from "./components/AgentConsole";
import { ActivityLog, type LogEntry } from "./components/ActivityLog";

const DEFAULT_ENDPOINT =
  import.meta.env.VITE_RPC_ENDPOINT ?? "https://api.devnet.solana.com";

export function App() {
  const endpoint = useMemo(() => DEFAULT_ENDPOINT, []);
  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={[]} autoConnect>
        <WalletModalProvider>
          <Dashboard endpoint={endpoint} />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

function Dashboard({ endpoint }: { endpoint: string }) {
  const { connection } = useConnection();
  const wallet = useWallet();

  const [mintInput, setMintInput] = useState("");
  const [decimals, setDecimals] = useState(6);
  const [delegation, setDelegation] = useState<DelegationAccount | null>(null);
  const [vaultBalance, setVaultBalance] = useState<bigint | null>(null);
  const [session, setSession] = useState<Keypair | null>(() => loadSession());
  const [sessionSol, setSessionSol] = useState<number | null>(null);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [busy, setBusy] = useState(false);

  const mint = useMemo(() => {
    try {
      return mintInput ? new PublicKey(mintInput) : null;
    } catch {
      return null;
    }
  }, [mintInput]);

  const owner = wallet.publicKey;
  const vault = useMemo(() => (owner ? aw.vaultPda(owner) : null), [owner]);

  const vaultAta = useMemo(
    () => (vault && mint ? getAssociatedTokenAddressSync(mint, vault, true) : null),
    [vault, mint],
  );
  const ownerAta = useMemo(
    () => (owner && mint ? getAssociatedTokenAddressSync(mint, owner) : null),
    [owner, mint],
  );

  // ## Logging

  const append = useCallback((entry: Omit<LogEntry, "id" | "at">) => {
    setLog((prev) => [
      { ...entry, id: crypto.randomUUID(), at: new Date() },
      ...prev,
    ]);
  }, []);

  // ## Reading chain state
  //
  // Re-read after every action rather than tracking locally. If the UI and the
  // chain disagree, the chain is right and the UI is the bug.

  const refresh = useCallback(async () => {
    if (!vault) return;
    try {
      if (session) {
        const d = await connection.getAccountInfo(
          aw.delegationPda(vault, session.publicKey),
        );
        setDelegation(d ? decodeDelegation(Buffer.from(d.data)) : null);
        setSessionSol(
          (await connection.getBalance(session.publicKey)) / LAMPORTS_PER_SOL,
        );
      }
      if (vaultAta) {
        const bal = await connection
          .getTokenAccountBalance(vaultAta)
          .catch(() => null);
        setVaultBalance(bal ? BigInt(bal.value.amount) : null);
        if (bal) setDecimals(bal.value.decimals);
      }
    } catch (err) {
      append({ kind: "error", title: "Could not read chain state", detail: String(err) });
    }
  }, [connection, vault, vaultAta, session, append]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // ## Sending

  const sendAsOwner = useCallback(
    async (label: string, instructions: Parameters<Transaction["add"]>) => {
      if (!wallet.publicKey || !wallet.sendTransaction) return;
      setBusy(true);
      try {
        const tx = new Transaction().add(...instructions);
        const sig = await wallet.sendTransaction(tx, connection);
        await connection.confirmTransaction(sig, "confirmed");
        append({ kind: "owner", title: label, signature: sig });
      } catch (err) {
        const decoded = decodeError(err);
        append({
          kind: "error",
          title: `${label} failed`,
          detail: decoded.message,
          errorName: decoded.name ?? undefined,
          errorCode: decoded.code ?? undefined,
        });
      } finally {
        setBusy(false);
        void refresh();
      }
    },
    [wallet, connection, append, refresh],
  );

  /**
   * Send signed by the agent's session key. The agent pays its own fees - the
   * vault's SOL is off limits, and the lamport floor in the program enforces
   * that even for an arbitrary CPI.
   */
  const sendAsAgent = useCallback(
    async (label: string, instruction: Parameters<Transaction["add"]>[0], hostile = false) => {
      if (!session) return;
      setBusy(true);
      try {
        const tx = new Transaction().add(instruction);
        tx.feePayer = session.publicKey;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
        tx.sign(session);
        const sig = await connection.sendRawTransaction(tx.serialize());
        await connection.confirmTransaction(sig, "confirmed");
        append({
          kind: hostile ? "breach" : "agent",
          title: label,
          signature: sig,
        });
      } catch (err) {
        const decoded = decodeError(err);
        append({
          kind: decoded.isPolicyRejection ? "blocked" : "error",
          title: label,
          detail: decoded.message,
          errorName: decoded.name ?? undefined,
          errorCode: decoded.code ?? undefined,
        });
      } finally {
        setBusy(false);
        void refresh();
      }
    },
    [session, connection, append, refresh],
  );

  // ## Owner actions

  const createVault = () =>
    owner && sendAsOwner("Vault created", [aw.initializeVault(owner)]);

  const deposit = (amount: bigint) =>
    owner && mint && ownerAta && vaultAta
      ? sendAsOwner("Deposited to vault", [
          aw.deposit(owner, mint, ownerAta, vaultAta, amount),
        ])
      : undefined;

  const grant = (policy: aw.Policy) => {
    if (!owner || !mint || !session) return;
    void sendAsOwner("Budget granted to agent", [
      aw.createDelegation(owner, session.publicKey, mint, policy),
    ]);
  };

  const update = (policy: aw.Policy) => {
    if (!owner || !session) return;
    void sendAsOwner("Policy updated", [
      aw.updateDelegation(owner, session.publicKey, policy),
    ]);
  };

  const revoke = () => {
    if (!owner || !session) return;
    void sendAsOwner("Delegation revoked", [
      aw.revokeDelegation(owner, session.publicKey),
    ]);
  };

  const fundSession = () => {
    if (!owner || !session) return;
    void sendAsOwner("Funded the agent's session key", [
      SystemProgram.transfer({
        fromPubkey: owner,
        toPubkey: session.publicKey,
        lamports: 0.02 * LAMPORTS_PER_SOL,
      }),
    ]);
  };

  const newSession = () => {
    clearSession();
    const kp = createSession();
    setSession(kp);
    setDelegation(null);
    append({
      kind: "owner",
      title: "New session key generated",
      detail: kp.publicKey.toBase58(),
    });
  };

  // ## Agent actions

  const agentCtx: aw.AgentContext | null =
    session && vault && mint && vaultAta
      ? { agent: session.publicKey, vault, mint, vaultTokenAccount: vaultAta }
      : null;

  const agentPay = (destination: PublicKey, amount: bigint) => {
    if (!agentCtx) return;
    void sendAsAgent(
      `Agent paid ${amount}`,
      aw.agentTransfer(agentCtx, destination, amount),
    );
  };

  /** The demo's turn: a hostile instruction the agent was talked into. */
  const runInjection = (attackerAta: PublicKey, drainAmount: bigint) => {
    if (!agentCtx) return;
    void sendAsAgent(
      "Agent attempted to drain the vault",
      aw.agentTransfer(agentCtx, attackerAta, drainAmount),
      true,
    );
  };

  /** The subtler attack: grant standing authority, take the funds later. */
  const runApprovalAttack = (attacker: PublicKey) => {
    if (!agentCtx) return;
    void sendAsAgent(
      "Agent attempted to approve an outside spender",
      aw.agentInvoke(agentCtx, TOKEN_PROGRAM_ID, aw.splApproveData(2n ** 63n), [
        { pubkey: agentCtx.vaultTokenAccount, isSigner: false, isWritable: true },
        { pubkey: attacker, isSigner: false, isWritable: false },
        { pubkey: agentCtx.vault, isSigner: false, isWritable: false },
      ]),
      true,
    );
  };

  return (
    <div className="app">
      <header className="topbar">
        <div>
          <h1>Agent Wallet</h1>
          <p className="tagline">Give an agent a budget, not your keys.</p>
        </div>
        <div className="topbar-right">
          <span className="endpoint" title={endpoint}>
            {new URL(endpoint).host}
          </span>
          <WalletMultiButton />
        </div>
      </header>

      {!owner ? (
        <div className="empty">
          <h2>Connect the owner wallet</h2>
          <p>
            This wallet funds the vault and sets the policy. It is never shared
            with the agent.
          </p>
        </div>
      ) : (
        <main className="grid">
          <div className="column">
            <VaultSetup
              vault={vault}
              mintInput={mintInput}
              onMintChange={setMintInput}
              mintValid={mint !== null}
              vaultAta={vaultAta}
              vaultBalance={vaultBalance}
              decimals={decimals}
              busy={busy}
              onCreateVault={createVault}
              onDeposit={deposit}
            />
            <PolicyEditor
              decimals={decimals}
              delegation={delegation}
              disabled={busy || !mint || !session}
              onGrant={grant}
              onUpdate={update}
              onRevoke={revoke}
            />
          </div>

          <div className="column">
            <AgentConsole
              session={session}
              sessionSol={sessionSol}
              delegation={delegation}
              decimals={decimals}
              busy={busy}
              onNewSession={newSession}
              onFundSession={fundSession}
              onPay={agentPay}
              onInject={runInjection}
              onApprovalAttack={runApprovalAttack}
            />
            <ActivityLog entries={log} />
          </div>
        </main>
      )}
    </div>
  );
}
