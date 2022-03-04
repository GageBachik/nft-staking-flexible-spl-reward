import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { TokenStakeModel } from '../target/types/token_stake_model';
import { TOKEN_PROGRAM_ID, Token, ASSOCIATED_TOKEN_PROGRAM_ID, } from '@solana/spl-token';
import { assert } from "chai";
import invariant from "tiny-invariant";
import { PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { BalanceTree } from "./balance-tree";

describe('token-stake-model', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenStakeModel as Program<TokenStakeModel>;

  let mintNFT = null; 
  let lpTokenMint = null;
  let nft_vault_pda = null;
  let lp_token_pda = null;
  let user_stake_pda = null;
  let nft_vault_bump = null;
  let lp_token_bump = null;
  let user_stake_bump = null;
  let userNftTokenAccount = null;
  let userLpTokenAccount = null;
  let nft_auth_pda = null;
  let nft_auth_bump = null;
  let merkle_pda = null;
  let merkle_bump = null;

  let leaves: {account: PublicKey}[] = [];
  let tree = null;
  let merkle_hash = null;
  let nftArray = [];

  const payer = anchor.web3.Keypair.generate();
  const nftAuthority = anchor.web3.Keypair.generate();
  const userAccount = anchor.web3.Keypair.generate();
  const treasuryAccount = new PublicKey("75Anj2RvhC5j8b2DniGoPSotBcst88fMt6Yo8xLATYJA");

  it('Is initialized!', async () => {

    // Airdrop 1 SOL to payer
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 2000000000),
      "confirmed"
    ); 

    await provider.send(
      (() => {
        const tx = new Transaction();
        tx.add(
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: userAccount.publicKey,
            lamports: 1000000000,
          }),
        );
        return tx;
      })(),
      [payer]
    );

    // Get the authority of NFT
    // [nft_auth_pda, nft_auth_bump] = await PublicKey.findProgramAddress([
    //   Buffer.from("vault-stake-auth"),
    // ], program.programId);

    // console.log("vault-stake-auth account", nft_auth_pda.toString());

    // [lp_token_pda, lp_token_bump] = await PublicKey.findProgramAddress([
    //   Buffer.from("reward-stake-auth"),
    // ], program.programId);

    // console.log("vault-stake-auth account", lp_token_pda.toString());

    // Create mint nft address; decimal = 0
    // mintNFT = await Token.createMint(
    //   provider.connection,
    //   payer,
    //   nftAuthority.publicKey,
    //   null,
    //   0,
    //   TOKEN_PROGRAM_ID,
    // );
    
    // Set the Decimal of Token
    // lpTokenMint = await Token.createMint(
    //   provider.connection,
    //   payer,
    //   lp_token_pda,
    //   null,
    //   6, // Decimal is 6
    //   TOKEN_PROGRAM_ID,
    // );

    // console.log("Lp token Mint", lpTokenMint.publicKey.toString());

    // // Create token account which can get the NFT
    // userNftTokenAccount = await mintNFT.createAccount(userAccount.publicKey);

    // // // Create the 1 NFT to user account
    // await mintNFT.mintTo(
    //   userNftTokenAccount,
    //   nftAuthority.publicKey,
    //   [nftAuthority],
    //   1
    // );

    // // Get the pda for vault account which have NFT
    // [nft_vault_pda, nft_vault_bump] = await PublicKey.findProgramAddress([
    //   Buffer.from("vault-stake"),
    //   mintNFT.publicKey.toBuffer(),
    //   userAccount.publicKey.toBuffer(),
    // ], program.programId);

    // // Get the account which have info of staking NFT
    [user_stake_pda, user_stake_bump] = await PublicKey.findProgramAddress([
      Buffer.from("user-stake"),
      userAccount.publicKey.toBuffer(),
    ], program.programId);


    await program.rpc.initialize(
      user_stake_bump,
      {
        accounts: {
          userAccount: userAccount.publicKey,
          stakeInfoAccount: user_stake_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAccount]
      }
    );

  });

  it('Initialize Merkle tree!', async () => {
    // [merkle_pda, merkle_bump] = await PublicKey.findProgramAddress([
    //   Buffer.from("Merkle"),
    //   payer.publicKey.toBuffer(),
    // ], program.programId);

    // console.log('merle pda', merkle_pda.toString());

    // let nftList = [
    //   "BfnPHQtCzy3ZVnPbWAPwyqEjZofZGirCACZcHrenHSJi",
    //   "XJruJeFYHbvRPFVgEQkxbowZyF6nxznDLEscx7Wxs4D",
    //   "365sECzzHZy3hcojKdZXsrffURZgcqBdsLMCakj2zym2",
    //   "2ERSbbmiCuh9rwcjS44dCQ5jvCQh43n5QRxSX3JoMJ91",
    //   "8spWVK1GJuDBQjHmwZjjCP9T7GUpqwNX9CEtrUXCN343",
    //   "CJBvJuLdk5357UjcVkN8pP2ECXHtEc7wGkikoh8h7VHf",
    //   "8aVnHKnedtLAxABDvBggZvUwwTQrEK75gb2h3rutKXoi",
    //   "Do9D3NaXe833EA6VHNnw2KtBtiZJJeA8fpHo8WKdcCdJ",
    //   "JDAZoJFrgjKAN4vJtPdG8dzp1YAZw8PXqMDQUpKRFhcL",
    //   "HfRtbmpw1SH8nheQmMxj1bWRnqbQ4b78GmaEaY4ozhed"
    // ];
    // for(var i = 0;i<nftList.length;i++) {
    //   nftArray.push({account: new PublicKey(nftList[i])})
    // }

    // nftArray.map(x => leaves.push(x));
    // tree = new BalanceTree(leaves);
    // merkle_hash = tree.getRoot();

    // console.log("mrekle tree", tree)
    
    
    // await program.rpc.initializeMerkle(
    //   merkle_bump,
    //   toBytes32Array(merkle_hash),
    //   {
    //     accounts: {
    //       adminAccount: payer.publicKey,
    //       merkle: merkle_pda,
    //       systemProgram: anchor.web3.SystemProgram.programId,
    //     },
    //     signers: [payer]
    //   }
    // );
  });

  // it('Stake NFT', async () => {
  //   const proof = tree.getProof(nftArray[1]['account']);
  //   await program.rpc.stakeNft(
  //     nft_vault_bump,
  //     nft_auth_bump,
  //     proof,
  //     {
  //       accounts: {
  //         userAccount: userAccount.publicKey,
  //         userNftTokenAccount: userNftTokenAccount,
  //         nftMint: mintNFT.publicKey,
  //         nftVaultAccount: nft_vault_pda,
  //         nftAuthority: nft_auth_pda,
  //         stakeInfoAccount: user_stake_pda,
  //         merkle: merkle_pda,
  //         treasuryAccount: treasuryAccount.publicKey,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //         rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [userAccount]
  //     }
  //   );

  //   let _userNFTAccount = await mintNFT.getAccountInfo(userNftTokenAccount);
  //   assert.ok(_userNFTAccount.amount.toNumber() == 0);

  //   let _vault = await mintNFT.getAccountInfo(nft_vault_pda);
  //   assert.ok(_vault.amount.toNumber() == 1);
  // });

  // it('Get All stacked NFTs', async () => {
    
  // });

  // it('Unstake NFT', async () => {
  //   const proof = tree.getProof(nftArray[1]['account']);
  //   await program.rpc.unstakeNft(
  //     proof,
  //     {
  //       accounts: {
  //         userAccount: userAccount.publicKey,
  //         userNftTokenAccount: userNftTokenAccount,
  //         nftMint: mintNFT.publicKey,
  //         nftVaultAccount: nft_vault_pda,
  //         stakeInfoAccount: user_stake_pda,
  //         vaultAuth: nft_auth_pda,
  //         merkle: merkle_pda,
  //         treasuryAccount: treasuryAccount.publicKey,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //       },
  //       signers: [userAccount]
  //     }
  //   );

  //   let _userNFTAccount = await mintNFT.getAccountInfo(userNftTokenAccount);
  //   assert.ok(_userNFTAccount.amount.toNumber() == 1);

  //   let _vault = await mintNFT.getAccountInfo(nft_vault_pda);
  //   assert.ok(_vault.amount.toNumber() == 0);
  // });

  it('Claim Reward token', async () => {
    // Create lp token account to User
    // const res = await Promise.all([
    //   // get token account, we don"t need to check if connected wallet has token account, because they have to pay with this token account
    //   Token.getAssociatedTokenAddress(
    //     ASSOCIATED_TOKEN_PROGRAM_ID,
    //     TOKEN_PROGRAM_ID,
    //     new PublicKey("BwiG6YoV443Gts88hm4RH9nUTF2zf1PYWF766RxYjKuA"),
    //     userAccount.publicKey
    //   ),
    // ]);

    // userLpTokenAccount = res[0];


    [user_stake_pda, user_stake_bump] = await PublicKey.findProgramAddress([
      Buffer.from("user-stake"),
      userAccount.publicKey.toBuffer(),
    ], program.programId);

    [lp_token_pda, lp_token_bump] = await PublicKey.findProgramAddress([
      Buffer.from("reward-stake-auth"),
    ], program.programId);

    lpTokenMint = await Token.createMint(
      provider.connection,
      payer,
      lp_token_pda,
      null,
      6, // Decimal is 6
      TOKEN_PROGRAM_ID,
    );

    userLpTokenAccount = await lpTokenMint.createAccount(userAccount.publicKey);

    console.log(userAccount.publicKey.toString())

    await program.rpc.claimReward(
      new anchor.BN(0),
      {
        accounts: {
          userAccount: userAccount.publicKey,
          lpTokenMint: lpTokenMint.publicKey,
          userLpAccount: userLpTokenAccount,
          stakeInfoAccount: user_stake_pda,
          lpTokenAuthority: lp_token_pda,
          treasuryAccount: new PublicKey("75Anj2RvhC5j8b2DniGoPSotBcst88fMt6Yo8xLATYJA"),
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAccount]
      }
    );
    
  });
});

const toBytes32Array = (b: Buffer): number[] => {
  invariant(b.length <= 32, `invalid length ${b.length}`);
  const buf = Buffer.alloc(32);
  b.copy(buf, 32 - b.length);

  return Array.from(buf);
};
