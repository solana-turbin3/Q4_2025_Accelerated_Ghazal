import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Escrow } from "../target/types/escrow";
import { SolanaGptOracle } from "../target/types/solana_gpt_oracle";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  getAccount,
  mintTo,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Keypair, SystemProgram, PublicKey } from "@solana/web3.js";
import { assert } from "chai";

// This test uses the real oracle callback path.
// Prereqs:
// - solana-gpt-oracle deployed and ORACLE_IDENTITY matches the llm_oracle runner signer
// - llm_oracle running with OPENAI_API_KEY and funded payer

describe("escrow dispute flow (real oracle)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const escrow = anchor.workspace.Escrow as Program<Escrow>;
  const oracle = anchor.workspace.SolanaGptOracle as Program<SolanaGptOracle>;

  const maker = Keypair.generate();
  const taker = Keypair.generate();

  const seed = 9;
  const depositA = 10_000;
  const takerPayB = 5_000;

  let mintA: PublicKey;
  let mintB: PublicKey;
  let makerAtaA: PublicKey;
  let takerAtaA: PublicKey;
  let makerAtaB: PublicKey;
  let takerAtaB: PublicKey;
  let escrowPda: PublicKey;
  let vaultA: PublicKey;
  let contextPda: PublicKey;
  let interactionPda: PublicKey;

  it("setup oracle context", async () => {
    // airdrop wallet + actors
    for (const pk of [provider.wallet.publicKey, maker.publicKey, taker.publicKey]) {
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(pk, 2 * anchor.web3.LAMPORTS_PER_SOL)
      );
    }

    // ensure oracle initialized
    const counterPda = PublicKey.findProgramAddressSync(
      [Buffer.from("counter")],
      oracle.programId
    )[0];
    try {
      await oracle.account.counter.fetch(counterPda);
    } catch (_) {
      const identityPda = PublicKey.findProgramAddressSync(
        [Buffer.from("identity")],
        oracle.programId
      )[0];
      await oracle.methods
        .initialize()
        .accounts({
          payer: provider.wallet.publicKey,
          identity: identityPda,
          counter: counterPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const counter = await oracle.account.counter.fetch(counterPda);
    contextPda = PublicKey.findProgramAddressSync(
      [Buffer.from("test-context"), new BN(counter.count).toArrayLike(Buffer, "le", 4)],
      oracle.programId
    )[0];

    try {
      await oracle.account.contextAccount.fetch(contextPda);
    } catch (_) {
      await oracle.methods
        .createLlmContext("Escrow dispute context")
        .accounts({
          payer: provider.wallet.publicKey,
          counter: counterPda,
          contextAccount: contextPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }
  });

  it("lock funds in escrow (make)", async () => {
    mintA = await createMint(provider.connection, maker, maker.publicKey, null, 6);
    mintB = await createMint(provider.connection, taker, taker.publicKey, null, 6);

    makerAtaA = (
      await getOrCreateAssociatedTokenAccount(provider.connection, maker, mintA, maker.publicKey)
    ).address;
    takerAtaA = (
      await getOrCreateAssociatedTokenAccount(provider.connection, taker, mintA, taker.publicKey)
    ).address;
    makerAtaB = (
      await getOrCreateAssociatedTokenAccount(provider.connection, maker, mintB, maker.publicKey)
    ).address;
    takerAtaB = (
      await getOrCreateAssociatedTokenAccount(provider.connection, taker, mintB, taker.publicKey)
    ).address;

    await mintTo(provider.connection, maker, mintA, makerAtaA, maker, depositA);
    await mintTo(provider.connection, taker, mintB, takerAtaB, taker, takerPayB * 2);

    escrowPda = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), maker.publicKey.toBuffer(), new BN(seed).toArrayLike(Buffer, "le", 8)],
      escrow.programId
    )[0];
    vaultA = getAssociatedTokenAddressSync(mintA, escrowPda, true);

    await escrow.methods
      .make(new BN(seed), new BN(takerPayB), new BN(depositA))
      .accounts({
        maker: maker.publicKey,
        mintA,
        mintB,
        makerAtaA,
        escrow: escrowPda,
        vault: vaultA,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();
  });

  it("open dispute and wait for oracle resolution", async () => {
    interactionPda = PublicKey.findProgramAddressSync(
      [Buffer.from("interaction"), provider.wallet.publicKey.toBuffer(), contextPda.toBuffer()],
      oracle.programId
    )[0];

    await escrow.methods
      .openDispute("Decide winner: maker or taker.")
      .accounts({
        payer: provider.wallet.publicKey,
        escrow: escrowPda,
        maker: maker.publicKey,
        taker: taker.publicKey,
        mintA,
        mintB,
        vault: vaultA,
        interaction: interactionPda,
        contextAccount: contextPda,
        oracleProgram: oracle.programId,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    // Wait up to ~60s for oracle to resolve and close vault
    const start = Date.now();
    let winner: "maker" | "taker" | null = null;
    while (Date.now() - start < 60000) {
      const va = await provider.connection.getAccountInfo(vaultA);
      if (!va) {
        // Vault closed — verify balances; winner is whoever got depositA
        const mA = Number((await getAccount(provider.connection, makerAtaA)).amount);
        const tA = Number((await getAccount(provider.connection, takerAtaA)).amount);
        if (mA >= depositA) winner = "maker"; else if (tA >= depositA) winner = "taker";
        break;
      }
      await new Promise(r => setTimeout(r, 1500));
    }
    assert.isNotNull(winner, "oracle should resolve dispute and close vault");
  });
});

