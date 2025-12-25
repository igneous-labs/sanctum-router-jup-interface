use std::sync::atomic::{AtomicU64, Ordering};

use jupiter_amm_interface::AccountMap;
use sanctum_router_std::{
    solido_legacy_core::{
        self, Lido, ValidatorList, LIDO_STATE_ADDR, STSOL_MINT_ADDR, VALIDATOR_LIST_ADDR,
    },
    LIDO_WITHDRAW_STAKE_IX_SUFFIX_ACCS_LEN,
};

use crate::{
    errs::{invalid_data_err, require_more_updates},
    stake::traits::{StakeRouter, TryWithdrawStake},
    utils::{mk_stake_pool_label, try_get_acc},
};

pub type LidoRouter =
    sanctum_router_std::LidoRouter<fn(&[&[u8]], &[u8; 32]) -> Option<([u8; 32], u8)>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LidoStakeRouter {
    #[default]
    Uninit,
    Init(LidoRouter),
}

impl StakeRouter for LidoStakeRouter {
    fn program_id(&self) -> &[u8; 32] {
        &solido_legacy_core::PROGRAM_ID
    }

    fn prog_dep_label(&self) -> String {
        mk_stake_pool_label(&STSOL_MINT_ADDR.into(), &None)
    }

    fn main_state_key(&self) -> &[u8; 32] {
        &LIDO_STATE_ADDR
    }

    fn mint(&self) -> &[u8; 32] {
        &STSOL_MINT_ADDR
    }

    fn get_accounts_to_update(&self) -> Vec<[u8; 32]> {
        [LIDO_STATE_ADDR, VALIDATOR_LIST_ADDR].into()
    }

    fn update(&mut self, am: &AccountMap) -> anyhow::Result<()> {
        let [s, v] =
            [LIDO_STATE_ADDR, VALIDATOR_LIST_ADDR].map(|addr| try_get_acc(am, &addr.into()));
        let s = s?;
        let v = v?;

        let state = Lido::borsh_de(s.data.as_slice())
            .map_err(|_| invalid_data_err(&LIDO_STATE_ADDR.into()))?;
        let ValidatorList { entries, .. } = ValidatorList::deserialize(&v.data)
            .map_err(|_e| invalid_data_err(&VALIDATOR_LIST_ADDR.into()))?;

        let curr_epoch = match self {
            Self::Uninit => 0,
            Self::Init(LidoRouter { curr_epoch, .. }) => *curr_epoch,
        };
        *self = Self::Init(LidoRouter::new(
            &state,
            entries,
            curr_epoch,
            crate::pda::find_pda,
        ));

        Ok(())
    }

    fn underlying_liquidity(&self) -> Option<[u8; 32]> {
        Some(LIDO_STATE_ADDR)
    }

    fn update_curr_epoch(&mut self, ce: &AtomicU64) {
        match self {
            Self::Uninit => (),
            Self::Init(LidoRouter { curr_epoch, .. }) => *curr_epoch = ce.load(Ordering::Relaxed),
        }
    }
}

impl TryWithdrawStake for LidoStakeRouter {
    type WithdrawStake = LidoRouter;

    const WITHDRAW_STAKE_SUF_ACCS_LEN: usize = LIDO_WITHDRAW_STAKE_IX_SUFFIX_ACCS_LEN;

    fn try_withdraw_stake(&self) -> anyhow::Result<&Self::WithdrawStake> {
        match self {
            Self::Uninit => Err(require_more_updates(1)),
            Self::Init(s) => Ok(s),
        }
    }
}
