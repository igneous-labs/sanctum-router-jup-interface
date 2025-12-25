use std::sync::atomic::AtomicU64;

use jupiter_amm_interface::AccountMap;
use sanctum_router_std::{DepositStake, WithdrawStake};

pub trait StakeRouter {
    /// stake pool program ID
    fn program_id(&self) -> &[u8; 32];

    fn prog_dep_label(&self) -> String;

    /// The main account to fetched on first update
    fn main_state_key(&self) -> &[u8; 32];

    /// The associated mint
    /// This is wsol for the sanctum reserves
    fn mint(&self) -> &[u8; 32];

    fn get_accounts_to_update(&self) -> Vec<[u8; 32]>;

    fn update(&mut self, am: &AccountMap) -> anyhow::Result<()>;

    fn underlying_liquidity(&self) -> Option<[u8; 32]> {
        None
    }

    /// Some stake pools need to have their curr_epoch field updated
    /// from AmmContext.
    ///
    /// Default impl is assume this is not such a stake pool and no-op
    fn update_curr_epoch(&mut self, _curr_epoch: &AtomicU64) {}
}

pub trait TryWithdrawStake {
    type WithdrawStake: WithdrawStake;

    const WITHDRAW_STAKE_SUF_ACCS_LEN: usize;

    fn try_withdraw_stake(&self) -> anyhow::Result<&Self::WithdrawStake>;
}

pub trait TryDepositStake {
    type DepositStake: DepositStake;

    const DEPOSIT_STAKE_SUF_ACCS_LEN: usize;

    fn try_deposit_stake(&self) -> anyhow::Result<&Self::DepositStake>;
}
