use jupiter_amm_interface::AccountMap;
use sanctum_router_std::{
    sanctum_reserve_core::{
        Fee, FeeEnum, Pool, PoolUnstakeParams, ProtocolFee, FEE, POOL, POOL_SOL_RESERVES,
        PROTOCOL_FEE, UNSTAKE_PROGRAM,
    },
    NATIVE_MINT, RESERVE_DEPOSIT_STAKE_IX_SUFFIX_ACCS_LEN,
};

use crate::{
    errs::{invalid_data_err, require_more_updates},
    stake::traits::{StakeRouter, TryDepositStake},
    utils::try_get_acc,
};

pub type ReserveRouter =
    sanctum_router_std::ReserveRouter<fn(&[&[u8]], &[u8; 32]) -> Option<([u8; 32], u8)>>;

pub const fn prefund_params(reserve: &ReserveRouter) -> (PoolUnstakeParams, &FeeEnum) {
    (
        PoolUnstakeParams {
            pool_incoming_stake: reserve.pool_incoming_stake,
            sol_reserves_lamports: reserve.pool_sol_reserves,
        },
        &reserve.fee_account,
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ReserveStakeRouter {
    #[default]
    Uninit,
    Init(ReserveRouter),
}

impl StakeRouter for ReserveStakeRouter {
    fn program_id(&self) -> &[u8; 32] {
        &UNSTAKE_PROGRAM
    }

    fn prog_dep_label(&self) -> &str {
        // current v1 impl
        "unstake.it"
    }

    fn main_state_key(&self) -> &[u8; 32] {
        &POOL
    }

    fn mint(&self) -> &[u8; 32] {
        &NATIVE_MINT
    }

    fn get_accounts_to_update(&self) -> Vec<[u8; 32]> {
        [POOL, FEE, PROTOCOL_FEE, POOL_SOL_RESERVES].into()
    }

    fn update(&mut self, am: &AccountMap) -> anyhow::Result<()> {
        let [p, f, pf, psr] =
            [POOL, FEE, PROTOCOL_FEE, POOL_SOL_RESERVES].map(|addr| try_get_acc(am, &addr.into()));
        let p = p?;
        let f = f?;
        let pf = pf?;
        let psr = psr?;

        let pool =
            Pool::anchor_de(p.data.as_slice()).map_err(|_e| invalid_data_err(&POOL.into()))?;
        let fee = Fee::anchor_de(f.data.as_slice()).map_err(|_e| invalid_data_err(&FEE.into()))?;
        let protocol_fee = ProtocolFee::anchor_de(pf.data.as_slice())
            .map_err(|_e| invalid_data_err(&PROTOCOL_FEE.into()))?;
        let pool_sol_reserves = psr.lamports;

        *self = Self::Init(ReserveRouter::new(
            fee,
            &protocol_fee,
            &pool,
            pool_sol_reserves,
            crate::pda::find_pda,
        ));

        Ok(())
    }

    fn underlying_liquidity(&self) -> Option<[u8; 32]> {
        Some(POOL)
    }
}

impl TryDepositStake for ReserveStakeRouter {
    type DepositStake = ReserveRouter;

    const DEPOSIT_STAKE_SUF_ACCS_LEN: usize = RESERVE_DEPOSIT_STAKE_IX_SUFFIX_ACCS_LEN;

    fn try_deposit_stake(&self) -> anyhow::Result<&Self::DepositStake> {
        match self {
            Self::Uninit => Err(require_more_updates(1)),
            Self::Init(s) => Ok(s),
        }
    }
}
