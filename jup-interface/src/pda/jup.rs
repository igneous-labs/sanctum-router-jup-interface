use sanctum_router_std::SANCTUM_ROUTER_PROGRAM;

use crate::pda::find_pda;

pub(crate) fn find_stake_pool_amm_key(main_state_key: &[u8; 32]) -> Option<([u8; 32], u8)> {
    find_pda(&[main_state_key], &SANCTUM_ROUTER_PROGRAM)
}
