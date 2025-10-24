import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";

// Must match src/lib.rs declare_id!
const PROGRAM_ID = new PublicKey(
  "9rcdaF2bdQVq3TjrL756VqcZWWYgLdZXJX79soxNoUjr"
);

const le64 = (n: number | bigint) => {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n));
  return b;
};

async function airDropAndConfirm(connection: Connection, pubkey: PublicKey, lamports: number) {
  const sig = await connection.requestAirdrop(pubkey, lamports);
  const latest = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature: sig, ...latest });
}

async function main() {
  const connection = new Connection("http://127.0.0.1:8899", "confirmed");

  // Fee payer (also the contributor in this test) and maker
  const payer = Keypair.generate();
  const maker = Keypair.generate();

  await airDropAndConfirm(connection, payer.publicKey, 2 * LAMPORTS_PER_SOL);
  await airDropAndConfirm(connection, maker.publicKey, 2 * LAMPORTS_PER_SOL);
  console.log("Airdropped 2 SOL to payer & maker");

  // Create mint and ATAs
  const mint = await createMint(
    connection,
    payer,
    payer.publicKey,
    payer.publicKey,
    6,
  );
  console.log("Mint created", mint.toBase58());

  const contributorATA = (
    await getOrCreateAssociatedTokenAccount(connection, payer, mint, payer.publicKey)
  ).address;

  const makerATA = (
    await getOrCreateAssociatedTokenAccount(connection, payer, mint, maker.publicKey)
  ).address;

  await mintTo(connection, payer, mint, contributorATA, payer, 1_000_000_0); // 10 tokens (6 dp)
  console.log("Minted 10 tokens to contributor ATA");

  // PDAs
  const [fundraiser, fundraiserBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("fundraiser"), maker.publicKey.toBuffer()],
    PROGRAM_ID,
  );
  const vault = getAssociatedTokenAddressSync(mint, fundraiser, true);

  // Program-owned contributor state account (8 bytes)
  const contributorAccount = Keypair.generate();
  const rentLamports = await connection.getMinimumBalanceForRentExemption(8);
  {
    const createIx = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: contributorAccount.publicKey,
      lamports: rentLamports,
      space: 8,
      programId: PROGRAM_ID,
    });
    const tx = new Transaction().add(createIx);
    await sendAndConfirmTransaction(connection, tx, [payer, contributorAccount]);
    console.log("Created contributor state account", contributorAccount.publicKey.toBase58());
  }

  // Initialize fundraiser
  // Data: [disc=0, bump, amount_to_raise u64, current_amount u64(0), time_started u64(now), duration u8]
  const amountToRaise = 30_000_000n;
  const nowSec = BigInt(Math.floor(Date.now() / 1000));
  const durationDays = 30; // leave long, so contribute passes now
  {
    const data = Buffer.concat([
      Buffer.from([0]),
      Buffer.from([fundraiserBump]),
      le64(amountToRaise),
      le64(0n),
      le64(nowSec),
      Buffer.from([durationDays]),
    ]);

    const keys = [
      { pubkey: maker.publicKey, isSigner: true, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: fundraiser, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];
    const ix = new TransactionInstruction({ programId: PROGRAM_ID, keys, data });
    const tx = new Transaction().add(ix);
    const sig = await sendAndConfirmTransaction(connection, tx, [maker]);
    console.log("\nInitialized fundraiser; tx", sig);
    await dumpFundraiserState(connection, fundraiser);
  }

  // Contribute helper
  const contribute = async (amount: bigint) => {
    const data = Buffer.concat([Buffer.from([1]), le64(amount)]);
    const keys = [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: fundraiser, isSigner: false, isWritable: true },
      { pubkey: contributorAccount.publicKey, isSigner: false, isWritable: true },
      { pubkey: contributorATA, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];
    const ix = new TransactionInstruction({ programId: PROGRAM_ID, keys, data });
    const tx = new Transaction().add(ix);
    const sig = await sendAndConfirmTransaction(connection, tx, [payer]);
    console.log("\nContributed", amount.toString(), "tx", sig);
    const vaultBal = await connection.getTokenAccountBalance(vault);
    console.log("Vault balance", vaultBal.value.amount);
  };

  // Contribute twice
  await contribute(1_000_000n);
  await contribute(1_000_000n);

  // Checker – likely fails until target is met
  try {
    const data = Buffer.from([2]);
    const keys = [
      { pubkey: maker.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: fundraiser, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: makerATA, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ];
    const ix = new TransactionInstruction({ programId: PROGRAM_ID, keys, data });
    const tx = new Transaction().add(ix);
    const sig = await sendAndConfirmTransaction(connection, tx, [payer]);
    console.log("\nChecked contributions; tx", sig);
  } catch (e: any) {
    const code = extractCustomErrorCode(e);
    if (code === 1000) {
      console.log("\nChecker failed with expected TargetNotMet (1000)");
    } else {
      console.error("\nChecker failed with unexpected error", code, e?.message ?? e);
      process.exit(1);
    }
  }

  // Refund – requires fundraiser ended and target not met; likely fails without time warp
  try {
    const data = Buffer.from([3]);
    const keys = [
      { pubkey: payer.publicKey, isSigner: true, isWritable: true }, // contributor
      { pubkey: maker.publicKey, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: fundraiser, isSigner: false, isWritable: true },
      { pubkey: contributorAccount.publicKey, isSigner: false, isWritable: true },
      { pubkey: contributorATA, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];
    const ix = new TransactionInstruction({ programId: PROGRAM_ID, keys, data });
    const tx = new Transaction().add(ix);
    const sig = await sendAndConfirmTransaction(connection, tx, [payer]);
    console.log("\nRefunded contributions; tx", sig);
  } catch (e: any) {
    const code = extractCustomErrorCode(e);
    if (code === 1005) {
      console.log("\nRefund failed with expected FundraiserNotEnded (1005)");
    } else {
      console.error("\nRefund failed with unexpected error", code, e?.message ?? e);
      process.exit(1);
    }
  }

  // Final state logs
  const vaultBal = await connection.getTokenAccountBalance(vault);
  console.log("Final Vault balance", vaultBal.value.amount);
  const info = await connection.getAccountInfo(contributorAccount.publicKey);
  const contributed = info ? Number(info.data.readBigUInt64LE(0)) : 0;
  console.log("Contributor recorded amount", contributed.toString());
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

function extractCustomErrorCode(e: any): number | undefined {
  try {
    const logs: string[] = Array.isArray(e?.transactionLogs)
      ? e.transactionLogs
      : typeof e?.logs === "string"
      ? [e.logs]
      : [];
    const text = [e?.transactionMessage, e?.message, ...logs].filter(Boolean).join("\n");
    const m = text.match(/custom program error: 0x([0-9a-fA-F]+)/);
    if (!m) return undefined;
    const hex = m[1];
    return parseInt(hex, 16);
  } catch {
    return undefined;
  }
}

async function dumpFundraiserState(connection: Connection, fundraiser: PublicKey) {
  try {
    const info = await connection.getAccountInfo(fundraiser);
    if (!info) {
      console.log("Fundraiser account not found");
      return;
    }
    const d = Buffer.from(info.data);
    if (d.length < 90) {
      console.log("Fundraiser data too small:", d.length);
      return;
    }
    const amount_to_raise = Number(d.readBigUInt64LE(64));
    const current_amount = Number(d.readBigUInt64LE(72));
    const time_started = Number(d.readBigUInt64LE(80));
    const duration = d.readUInt8(88);
    const bump = d.readUInt8(89);
    console.log("Fundraiser state:", { amount_to_raise, current_amount, time_started, duration, bump });
  } catch (e) {
    console.log("Failed to dump fundraiser state", e);
  }
}
