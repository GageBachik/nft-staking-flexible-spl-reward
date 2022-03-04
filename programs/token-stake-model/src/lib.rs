use anchor_lang::{prelude::*, solana_program::clock};
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer, CloseAccount, Token};
use anchor_lang::solana_program::{program::invoke, system_instruction };
use std::mem::size_of;
use solana_program::{pubkey::Pubkey};

pub mod error;
use crate::{error::StakeError};

pub mod merkle_proof;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const STOP_STAKE_DATE: i64 = 1645747200; // 2022-02-25 00:00:00 GMT+0
const STAKE_FEE: u64 = 50000000; // 0.05SOL
const SPL_TOKENS_PER_SEC: u64 = 100; // 0.0001 SPL token, Decimal is 6  per
const TREASURY_ACCOUNT: &str = "75Anj2RvhC5j8b2DniGoPSotBcst88fMt6Yo8xLATYJA";
#[program]
pub mod token_stake_model {
    use super::*;
    pub fn initialize(
        ctx: Context<Initialize>,
        _user_nonce: u8,
    ) -> ProgramResult {
        if !ctx.accounts.user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let clock = clock::Clock::get().unwrap();
        if clock.unix_timestamp > STOP_STAKE_DATE {
            return Err(StakeError::NoStakeAnyMore.into());
        }
        if ctx.accounts.stake_info_account.is_initial {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        ctx.accounts.stake_info_account.user_account = ctx.accounts.user_account.key();
        ctx.accounts.stake_info_account.reward_amount = 0;
        ctx.accounts.stake_info_account.nft_count = 0;
        ctx.accounts.stake_info_account.is_initial = true;
        Ok(())
    }

    pub fn initialize_merkle(
        ctx: Context<InitializeMerkle>,
        nonce: u8,
        root: [u8; 32],
    ) -> ProgramResult {
        if ctx.accounts.merkle.is_init {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        let merkle = &mut ctx.accounts.merkle;
        merkle.bump = nonce;
        merkle.root = root;
        merkle.is_init = true;
        merkle.admin_account = ctx.accounts.admin_account.key();
        Ok(())
    }

    pub fn update_merkle(
        ctx: Context<UpdateMerkle>,
        root: [u8; 32],
    ) -> ProgramResult {
        if !ctx.accounts.merkle.is_init {
            return Err(ProgramError::UninitializedAccount);
        }
        let merkle = &mut ctx.accounts.merkle;
        merkle.root = root;
        Ok(())
    }

    pub fn stake_nft(
        ctx: Context<StakeNft>,
        _nft_vault_nonce: u8,
        _nft_auth_nonce: u8,
        proof: Vec<[u8; 32]>,
    ) -> ProgramResult {
        if !ctx.accounts.user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if **ctx.accounts.user_account.lamports.borrow() < STAKE_FEE {
            return Err(StakeError::NoEnoughSol.into());
        }
        let clock = clock::Clock::get().unwrap();

        if clock.unix_timestamp > STOP_STAKE_DATE {
            return Err(StakeError::NoStakeAnyMore.into());
        }

        let merkle_seed = b"nft-staking-merkle-tree";

        let node = anchor_lang::solana_program::keccak::hashv(&[
            &merkle_seed.as_ref(),
            &ctx.accounts.nft_mint.to_account_info().key().to_bytes(),
        ]);

        let merkle = &ctx.accounts.merkle;
        
        if !merkle_proof::verify(proof, merkle.root, node.0) {
            return Err(StakeError::InvalidProof.into());
        } 

        // transfer user's SOL to treasury account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.user_account.key,
                ctx.accounts.treasury_account.key,
                STAKE_FEE,
            ),
            &[
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.treasury_account.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        // transfer the nft to vault account
        token::transfer(
            ctx.accounts.into_transfer_to_pda_context(),
            1,
        )?;

        
        let cur_time = clock.unix_timestamp;
        let cur_reward_amount = ((cur_time - ctx.accounts.stake_info_account.stake_time)) as u64 * SPL_TOKENS_PER_SEC * ctx.accounts.stake_info_account.nft_count as u64;

        ctx.accounts.stake_info_account.stake_time = clock.unix_timestamp;
        ctx.accounts.stake_info_account.reward_amount += cur_reward_amount;
        ctx.accounts.stake_info_account.nft_count = ctx.accounts.stake_info_account.nft_count + 1;
        
        Ok(())
    }

    pub fn unstake_nft(
        ctx: Context<UnStakeNft>,
        proof: Vec<[u8; 32]>,
    ) -> ProgramResult {
        if !ctx.accounts.user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if **ctx.accounts.user_account.lamports.borrow() < STAKE_FEE {
            return Err(StakeError::NoEnoughSol.into());
        }
        let clock = clock::Clock::get().unwrap();

        let merkle_seed = b"nft-staking-merkle-tree";

        let node = anchor_lang::solana_program::keccak::hashv(&[
            &merkle_seed.as_ref(),
            &ctx.accounts.nft_mint.to_account_info().key().to_bytes(),
        ]);

        let merkle = &ctx.accounts.merkle;
        
        if !merkle_proof::verify(proof, merkle.root, node.0) {
            return Err(StakeError::InvalidProof.into());
        } 

        // transfer the nft to vault account
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[b"vault-stake-auth"], ctx.program_id);

        let authority_seeds = &[&b"vault-stake-auth"[..], &[vault_authority_bump]];

        token::transfer(
            ctx.accounts.into_transfer_to_user_context().with_signer(&[&authority_seeds[..]]),
            1,
        )?;

        // transfer user's SOL to treasury account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.user_account.key,
                ctx.accounts.treasury_account.key,
                STAKE_FEE,
            ),
            &[
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.treasury_account.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        
        let cur_time = clock.unix_timestamp;
        let cur_reward_amount = ((cur_time - ctx.accounts.stake_info_account.stake_time) ) as u64 * SPL_TOKENS_PER_SEC * ctx.accounts.stake_info_account.nft_count as u64;

        ctx.accounts.stake_info_account.stake_time = clock.unix_timestamp;
        ctx.accounts.stake_info_account.reward_amount += cur_reward_amount;
        ctx.accounts.stake_info_account.nft_count = ctx.accounts.stake_info_account.nft_count - 1;

        token::close_account(
            ctx.accounts
                .into_close_context()
                .with_signer(&[&authority_seeds[..]]),
        )?;

        Ok(())
    }

    pub fn claim_reward(
        ctx: Context<ClaimReward>,
        claim_amount: u64
    ) -> ProgramResult {
        if !ctx.accounts.user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if **ctx.accounts.user_account.lamports.borrow() < STAKE_FEE {
            return Err(StakeError::NoEnoughSol.into());
        }
        if claim_amount < 0 {
            return Err(StakeError::NotEnoughLP.into());
        }

        let clock = clock::Clock::get().unwrap();
        // Get current time
        let current_time = clock.unix_timestamp;
        // Get the period of staking
        let last_staking_days = current_time - ctx.accounts.stake_info_account.stake_time;

        // Get Reward amount
        let stake_lp_reward_amount = last_staking_days as u64 * SPL_TOKENS_PER_SEC * ctx.accounts.stake_info_account.nft_count as u64 + ctx.accounts.stake_info_account.reward_amount; // LP token decimal is 6

        // Check the lp amount
        if claim_amount > stake_lp_reward_amount {
            return Err(StakeError::NotEnoughLP.into());
        }

        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[b"reward-stake-auth"], ctx.program_id);

        let seeds = &[&b"reward-stake-auth"[..], &[vault_authority_bump]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.lp_token_mint.to_account_info(),
            to: ctx.accounts.user_lp_account.to_account_info(),
            authority: ctx.accounts.lp_token_authority.to_account_info(),
        };

        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
            ).with_signer(&[&seeds[..]]),
            claim_amount
        )?;

        // transfer user's SOL to treasury account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.user_account.key,
                ctx.accounts.treasury_account.key,
                STAKE_FEE,
            ),
            &[
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.treasury_account.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        ctx.accounts.stake_info_account.reward_amount = stake_lp_reward_amount - claim_amount;
        ctx.accounts.stake_info_account.stake_time = current_time;

        Ok(())
    }
}


#[derive(Accounts)]
#[instruction(user_nonce: u8)]
pub struct Initialize<'info> {
    // user who stack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        init,
        seeds = [ 
            b"user-stake".as_ref(),
            user_account.key().as_ref(),
        ],
        bump = user_nonce,
        payer = user_account,
        space = 8 + size_of::<StakeInfoAccount>()
    )]
    pub stake_info_account: Box<Account<'info, StakeInfoAccount>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct InitializeMerkle<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"Merkle".as_ref(),
            admin_account.key().to_bytes().as_ref()
        ],
        bump = nonce,
        payer = admin_account,
        space = 8 + size_of::<Merkle>()
    )]
    pub merkle: Box<Account<'info, Merkle>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMerkle<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        mut,
        has_one = admin_account
    )]
    pub merkle: Box<Account<'info, Merkle>>,
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
#[instruction(vault_nonce: u8,nft_auth_nonce: u8)]
pub struct StakeNft<'info> {
    // user who stack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        mut,
        constraint = user_nft_token_account.amount == 1 && user_nft_token_account.owner == user_account.to_account_info().key()
    )]
    pub user_nft_token_account: Box<Account<'info, TokenAccount>>,
    // NFT mint
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        seeds = [
            b"vault-stake".as_ref(),
            nft_mint.key().as_ref(),
            user_account.key().as_ref(),
        ],
        bump = vault_nonce,
        payer = user_account,
        token::mint = nft_mint,
        token::authority = nft_authority,
    )]
    pub nft_vault_account: Box<Account<'info, TokenAccount>>,
    // #[account(
    //     init,
    //     seeds = [
    //         b"vault-stake-auth".as_ref(),
    //     ],
    //     bump = nft_auth_nonce,
    //     payer = user_account,
    //     space = 8
    // )]
    pub nft_authority: AccountInfo<'info>,
    
    #[account(
        mut,
        has_one = user_account,
        constraint = user_account.key == &stake_info_account.user_account
    )]
    pub stake_info_account: Box<Account<'info, StakeInfoAccount>>,
    pub merkle: Box<Account<'info, Merkle>>,
    #[account(
        mut,
        constraint = treasury_account.key() == TREASURY_ACCOUNT.parse::<Pubkey>().unwrap()
    )]
    pub treasury_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UnStakeNft<'info> {
    // user who unstack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(mut)]
    pub user_nft_token_account: Box<Account<'info, TokenAccount>>,
    // NFT mint
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        constraint = nft_vault_account.mint == nft_mint.key(),
        constraint = nft_vault_account.owner == vault_auth.key()
    )]
    pub nft_vault_account: Box<Account<'info, TokenAccount>>,
    
    #[account(
        mut,
        has_one = user_account,
        constraint = user_account.key == &stake_info_account.user_account
    )]
    pub stake_info_account: Box<Account<'info, StakeInfoAccount>>,
    pub vault_auth: AccountInfo<'info>,
    pub merkle: Box<Account<'info, Merkle>>,
    #[account(
        mut,
        constraint = treasury_account.key() == TREASURY_ACCOUNT.parse::<Pubkey>().unwrap()
    )]
    pub treasury_account: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    // user account who stack NFT
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        constraint = user_lp_account.owner == user_account.key(),
        constraint = user_lp_account.mint == lp_token_mint.key()
    )]
    pub user_lp_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        has_one = user_account,
        constraint = user_account.key == &stake_info_account.user_account
    )]
    pub stake_info_account: Box<Account<'info, StakeInfoAccount>>,
    pub lp_token_authority: AccountInfo<'info>,
    #[account(
        mut,
        constraint = treasury_account.key() == TREASURY_ACCOUNT.parse::<Pubkey>().unwrap()
    )]
    pub treasury_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}


impl<'info> StakeNft<'info> {
    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .user_nft_token_account
                .to_account_info()
                .clone(),
            to: self.nft_vault_account.to_account_info().clone(),
            authority: self.user_account.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

impl<'info> UnStakeNft<'info> {
    fn into_transfer_to_user_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .nft_vault_account
                .to_account_info()
                .clone(),
            to: self.user_nft_token_account.to_account_info().clone(),
            authority: self.vault_auth.clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn into_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.nft_vault_account.to_account_info().clone(),
            destination: self.user_account.to_account_info().clone(),
            authority: self.vault_auth.clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[account]
pub struct StakeInfoAccount {
    pub user_account: Pubkey,
    pub stake_time: i64,
    pub reward_amount: u64,
    pub nft_count: u16,
    pub is_initial: bool
}

#[account]
#[derive(Default)]
pub struct Merkle {
    pub bump: u8,
    /// The 256-bit merkle root.
    pub root: [u8; 32],
    pub admin_account: Pubkey,
    pub is_init: bool
}