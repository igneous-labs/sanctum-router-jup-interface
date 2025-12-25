use sanctum_router_std::{
    bridge_stake_seeds, fee_token_acc_seeds, SANCTUM_ROUTER_PROGRAM, SLUMDOG_SEED, STAKE_PROGRAM,
};
use solana_pubkey::Pubkey;

use crate::pda::find_pda;

pub(crate) fn find_fee_token_account_pda(mint: &[u8; 32]) -> Option<([u8; 32], u8)> {
    let (s1, s2) = fee_token_acc_seeds(mint);
    find_pda(&[s1.as_slice(), s2.as_slice()], &SANCTUM_ROUTER_PROGRAM)
}

pub(crate) fn find_bridge_stake_pda(
    user: &[u8; 32],
    bridge_stake_seed: u32,
) -> Option<([u8; 32], u8)> {
    let (s1, s2, s3) = bridge_stake_seeds(user, bridge_stake_seed);
    find_pda(
        &[s1.as_slice(), s2.as_slice(), s3.as_slice()],
        &SANCTUM_ROUTER_PROGRAM,
    )
}

pub(crate) fn create_slumdog_stake_addr(bridge_stake: &[u8; 32]) -> [u8; 32] {
    // unwrap-safety:
    // - seed.len() <= MAX_SEED_LEN
    // - Stake program ID's last bytes are not PDA_MARKER
    Pubkey::create_with_seed(&(*bridge_stake).into(), SLUMDOG_SEED, &STAKE_PROGRAM.into())
        .unwrap()
        .to_bytes()
}
