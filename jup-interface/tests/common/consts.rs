use std::sync::{atomic::AtomicU64, Arc};

use jupiter_amm_interface::{AmmContext, ClockRef};
use lazy_static::lazy_static;
use solana_pubkey::Pubkey;
use test_utils::TEST_EPOCH;

lazy_static! {
    pub static ref AMM_CONTEXT: AmmContext = {
        AmmContext {
            clock_ref: ClockRef {
                epoch: Arc::new(AtomicU64::new(TEST_EPOCH)),
                ..Default::default()
            },
        }
    };
}

// use these pubkeys for consistent, easily identifiable addrs

pub const TEST_SIGNER: Pubkey = Pubkey::from_str_const("11111111111111111111111111111112");
pub const TOKEN_ACC_1: Pubkey = Pubkey::from_str_const("11111111111111111111111111111113");
pub const TOKEN_ACC_2: Pubkey = Pubkey::from_str_const("11111111111111111111111111111114");
