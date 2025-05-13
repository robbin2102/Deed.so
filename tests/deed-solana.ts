import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DeedPreTge } from "../target/types/deed_pre_tge";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert } from "chai";

describe("deed-pre-tge", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.DeedPreTge as Program<DeedPreTge>;

  const wallet = provider.wallet;
  const creator = wallet;
  const buyer = wallet;
  const referrer = Keypair.generate();

  const salePda = PublicKey.findProgramAddressSync(
    [Buffer.from("sale"), creator.publicKey.toBuffer()],
    program.programId
  )[0];

  const saleVault = Keypair.generate();
  const rewardVault = Keypair.generate();

  it("Initializes the sale", async () => {
    const maxSupply = new anchor.BN(1000);
    const tiers = [
      { amount: new anchor.BN(500), price: new anchor.BN(1_000_000) },
      { amount: new anchor.BN(500), price: new anchor.BN(2_000_000) },
    ];
    const endDate = new anchor.BN(Math.floor(Date.now() / 1000) + 3600);

    await program.methods
      .initializeSale(maxSupply, tiers, endDate)
      .accounts({
        sale: salePda,
        creator: creator.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const saleAccount = await program.account.tokenSale.fetch(salePda);
    assert.equal(saleAccount.maxSupply.toNumber(), 1000);
  });

  it("Buys tokens", async () => {
    const amount = new anchor.BN(100);
    const balanceBefore = await provider.connection.getBalance(rewardVault.publicKey);

    await program.methods
      .buyTokens(amount, referrer.publicKey)
      .accounts({
        sale: salePda,
        saleVault: saleVault.publicKey,
        rewardVault: rewardVault.publicKey,
        buyer: buyer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const balanceAfter = await provider.connection.getBalance(rewardVault.publicKey);
    assert.equal(balanceAfter - balanceBefore, 25_000_000); // 25% of 0.1 SOL
  });

  it("Claims rewards", async () => {
    const rewardAmount = new anchor.BN(10_000_000); // 0.01 SOL
    const referrerBalanceBefore = await provider.connection.getBalance(referrer.publicKey);

    await program.methods
      .claimRewards(rewardAmount)
      .accounts({
        sale: salePda,
        rewardVault: rewardVault.publicKey,
        referrer: referrer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([referrer]) // Referrer signs and receives
      .rpc();

    const referrerBalanceAfter = await provider.connection.getBalance(referrer.publicKey);
    assert.equal(referrerBalanceAfter - referrerBalanceBefore, 10_000_000);
  });

  
});