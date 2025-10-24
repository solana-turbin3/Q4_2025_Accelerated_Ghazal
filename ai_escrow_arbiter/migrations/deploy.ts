// Migrations are an early feature. Currently, they're nothing more than this
// single deploy script that's invoked from the CLI, injecting a provider
// configured from the workspace's Anchor.toml.

const anchor = require("@coral-xyz/anchor");

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  const wallet = provider.wallet;
  const pgOracle = anchor.workspace.SolanaGptOracle;
  const pgAgent = anchor.workspace.SimpleAgent;

  // Idempotent helper: fetch or initialize oracle PDAs
  async function ensureOracleInitialized() {
    const counterPda = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("counter")],
      pgOracle.programId
    )[0];
    const identityPda = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("identity")],
      pgOracle.programId
    )[0];

    // Try fetching counter; if missing, run initialize
    try {
      await pgOracle.account.counter.fetch(counterPda);
      return { counterPda, identityPda };
    } catch (_) {}

    try {
      await pgOracle.methods
        .initialize()
        .accounts({
          payer: wallet.publicKey,
          identity: identityPda,
          counter: counterPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } catch (e) {
      // If already initialized, ignore
      const msg = String(e || "");
      if (!/already in use|0x0/i.test(msg)) throw e;
    }
    return { counterPda, identityPda };
  }

  // Idempotent helper: ensure SimpleAgent agent PDA exists (and a context)
  async function ensureAgentInitialized(counterPda) {
    // Fetch current counter to select a context PDA
    const counter = await pgOracle.account.counter.fetch(counterPda);
    const contextPda = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("test-context"), new anchor.BN(counter.count).toArrayLike(Buffer, "le", 4)],
      pgOracle.programId
    )[0];

    const agentPda = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("agent")],
      pgAgent.programId
    )[0];

    try {
      await pgAgent.account.agent.fetch(agentPda);
      return { agentPda, contextPda };
    } catch (_) {}

    try {
      await pgAgent.methods
        .initialize()
        .accounts({
          payer: wallet.publicKey,
          agent: agentPda,
          llmContext: contextPda,
          counter: counterPda,
          systemProgram: anchor.web3.SystemProgram.programId,
          oracleProgram: pgOracle.programId,
        })
        .rpc();
    } catch (e) {
      const msg = String(e || "");
      if (!/already in use|0x0/i.test(msg)) throw e;
    }
    return { agentPda, contextPda };
  }

  // Run steps
  const { counterPda } = await ensureOracleInitialized();
  await ensureAgentInitialized(counterPda);
};
