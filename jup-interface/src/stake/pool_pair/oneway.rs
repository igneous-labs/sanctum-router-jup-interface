use std::{
    collections::HashSet,
    sync::{atomic::AtomicU64, Arc},
};

use anyhow::anyhow;
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas,
    SwapMode, SwapParams,
};
use sanctum_router_std::{
    quote_prefund_swap_via_stake, DepositStake, WithdrawStake, DEPOSIT_STAKE_IX_ACCS_LEN,
    PREFUND_WITHDRAW_STAKE_PREFIX_ACCS_LEN, SANCTUM_ROUTER_PROGRAM,
};
use solana_pubkey::Pubkey;

use crate::{
    conv::conv_prefund_swap_via_stake_quote,
    errs::{exact_out_unsupported_err, invalid_pda, unsupported_mints},
    pda::router::find_fee_token_account_pda,
    prefund_params,
    stake::{
        pool_pair::common::{
            det_stake_pool_pair_amm_key, prep_underlying_liquidities,
            try_prefund_swap_via_stake_metas,
        },
        traits::{StakeRouter, TryDepositStake, TryWithdrawStake},
    },
    ReserveStakeRouter, TEMPORARY_JUP_AMM_LABEL,
};

#[derive(Clone)]
pub struct OneWayPair<W, D> {
    pub withdraw: W,
    pub deposit: D,
    pub reserve: ReserveStakeRouter,
    pub curr_epoch: Arc<AtomicU64>,

    // Cached PDAs below
    pub amm_key: [u8; 32],
    pub deposit_sr_fee_token_acc: [u8; 32],
}

impl<
        W: TryWithdrawStake + StakeRouter + Default + Clone + Send + Sync + 'static,
        D: TryDepositStake + StakeRouter + Default + Clone + Send + Sync + 'static,
    > Amm for OneWayPair<W, D>
{
    fn from_keyed_account(_ka: &KeyedAccount, ctx: &AmmContext) -> anyhow::Result<Self> {
        let withdraw = W::default();
        let deposit = D::default();
        let reserve = ReserveStakeRouter::default();
        let curr_epoch = ctx.clock_ref.epoch.clone();
        let amm_key = det_stake_pool_pair_amm_key(&withdraw, &deposit)?;
        let (deposit_sr_fee_token_acc, _) =
            find_fee_token_account_pda(deposit.mint()).ok_or_else(|| {
                invalid_pda(&format!(
                    "{} fee token account",
                    Pubkey::from(*deposit.mint())
                ))
            })?;
        Ok(Self {
            withdraw,
            deposit,
            reserve,
            curr_epoch,
            amm_key,
            deposit_sr_fee_token_acc,
        })
    }

    fn label(&self) -> String {
        TEMPORARY_JUP_AMM_LABEL.to_owned()
    }

    fn program_id(&self) -> Pubkey {
        SANCTUM_ROUTER_PROGRAM.into()
    }

    fn key(&self) -> Pubkey {
        self.amm_key.into()
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        [self.withdraw.mint(), self.deposit.mint()]
            .map(|a| (*a).into())
            .into()
    }

    fn unidirectional(&self) -> bool {
        true
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        self.withdraw
            .get_accounts_to_update()
            .into_iter()
            .chain(self.deposit.get_accounts_to_update())
            .chain(self.reserve.get_accounts_to_update())
            .map(Into::into)
            .collect()
    }

    fn update(&mut self, am: &AccountMap) -> anyhow::Result<()> {
        let w = self.withdraw.update(am);
        let d = self.deposit.update(am);
        let r = self.reserve.update(am);
        self.withdraw.update_curr_epoch(&self.curr_epoch);
        self.deposit.update_curr_epoch(&self.curr_epoch);
        self.reserve.update_curr_epoch(&self.curr_epoch);
        // try to update everything eagerly even if smth
        // fails in the middle, only returning err at the end
        w.and(d).and(r)
    }

    fn quote(
        &self,
        QuoteParams {
            amount,
            input_mint,
            output_mint,
            swap_mode,
        }: &QuoteParams,
    ) -> anyhow::Result<Quote> {
        if matches!(swap_mode, SwapMode::ExactOut) {
            return Err(exact_out_unsupported_err());
        }
        let [wmint, dmint] = [self.withdraw.mint(), self.deposit.mint()].map(|a| (*a).into());
        if *input_mint == wmint && *output_mint == dmint {
            let w = self.withdraw.try_withdraw_stake()?;
            let d = self.deposit.try_deposit_stake()?;
            let r = self.reserve.try_deposit_stake()?;
            let (rup, rf) = prefund_params(r);
            Ok(conv_prefund_swap_via_stake_quote(
                *output_mint,
                quote_prefund_swap_via_stake(
                    w.withdraw_stake_val_quoters(),
                    d.deposit_stake_quoter(),
                    *amount,
                    &rup,
                    rf,
                )
                .map_err(|e| anyhow!("{e}"))?,
            ))
        } else {
            Err(unsupported_mints(input_mint, output_mint))
        }
    }

    fn get_swap_and_account_metas(&self, sp: &SwapParams) -> anyhow::Result<SwapAndAccountMetas> {
        let [wmint, dmint] = [self.withdraw.mint(), self.deposit.mint()].map(|a| (*a).into());
        let SwapParams {
            source_mint,
            destination_mint,
            ..
        } = sp;
        if *source_mint == wmint && *destination_mint == dmint {
            let w = self.withdraw.try_withdraw_stake()?;
            let d = self.deposit.try_deposit_stake()?;
            let r = self.reserve.try_deposit_stake()?;
            let bridge_stake_seed = rand::random();
            let account_metas = try_prefund_swap_via_stake_metas(
                w,
                d,
                r,
                sp,
                bridge_stake_seed,
                &self.deposit_sr_fee_token_acc,
            )?;
            Ok(SwapAndAccountMetas {
                swap: Swap::StakeDexPrefundWithdrawStakeAndDepositStake { bridge_stake_seed },
                account_metas,
            })
        } else {
            Err(unsupported_mints(source_mint, destination_mint))
        }
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn get_accounts_len(&self) -> usize {
        PREFUND_WITHDRAW_STAKE_PREFIX_ACCS_LEN
            + W::WITHDRAW_STAKE_SUF_ACCS_LEN
            + 1
            + DEPOSIT_STAKE_IX_ACCS_LEN
            + D::DEPOSIT_STAKE_SUF_ACCS_LEN
    }

    fn program_dependencies(&self) -> Vec<(Pubkey, String)> {
        vec![
            (
                Pubkey::from(*self.withdraw.program_id()),
                self.withdraw.prog_dep_label().to_owned(),
            ),
            (
                Pubkey::from(*self.deposit.program_id()),
                self.deposit.prog_dep_label().to_owned(),
            ),
            (
                Pubkey::from(*self.reserve.program_id()),
                self.reserve.prog_dep_label().to_owned(),
            ),
        ]
    }

    fn underlying_liquidities(&self) -> Option<HashSet<Pubkey>> {
        prep_underlying_liquidities(&self.withdraw, &self.deposit)
    }
}
