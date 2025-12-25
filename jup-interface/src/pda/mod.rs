//! Interfaces all use `[u8; 32]` instead of [`Pubkey`] to minimize dependency on solana-pubkey.
//!
//! And also because our generic fn types are over `[u8; 32]`

use solana_pubkey::Pubkey;

pub mod jup;
pub mod reserve;
pub mod router;
pub mod spl;

pub(crate) fn find_pda(seeds: &[&[u8]], program_addr: &[u8; 32]) -> Option<([u8; 32], u8)> {
    Pubkey::try_find_program_address(seeds, &Pubkey::new_from_array(*program_addr))
        .map(|(addr, bump)| (addr.to_bytes(), bump))
}
