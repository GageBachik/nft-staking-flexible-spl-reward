use anchor_lang::prelude::*;

#[error]
pub enum StakeError {
    #[msg("Not enough LP token amount")]
    NotEnoughLP,
    #[msg("Whitelist Account is not initialized")]
    NotInit,
    #[msg("NFT address is not whitelisted")]
    InvalidToken,
    #[msg("Signer Not Token Whitelist Owner")]
    TokenWhitelistNotOwner,
    #[msg("Invalid Merkle proof.")]
    InvalidProof,
    #[msg("Staking day should be 7 days")]
    StakingDay,
    #[msg("You can't stake NFT any more")]
    NoStakeAnyMore,
    #[msg("You must pay the 0.05 SOL for stake or unstake")]
    NoEnoughSol
}
