use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas,
    SwapMode, SwapParams,
};
use sanctum_router_std::{
    sanctum_marinade_liquid_staking_core::{
        State, LIQ_POOL_MSOL_LEG_PUBKEY, MARINADE_STAKING_PROGRAM, MSOL_MINT_ADDR, STATE_PUBKEY,
    },
    DepositSol, DepositSolQuoter, DepositSolSufAccs, MarinadeRouterSol,
    MARINADE_DEPOSIT_SOL_IX_SUFFIX_ACCS_LEN, NATIVE_MINT, SANCTUM_ROUTER_PROGRAM,
    STAKE_WRAPPED_SOL_PREFIX_ACCS_LEN, STAKE_WRAPPED_SOL_PREFIX_IS_WRITER,
};
use solana_pubkey::Pubkey;

use crate::{
    conv::{conv_token_quote, keys_writable_zipped_to_metas},
    errs::{exact_out_unsupported_err, invalid_pda, require_more_updates, unsupported_mints},
    pda::{jup::find_stake_pool_amm_key, router::find_fee_token_account_pda},
    sol::common::stake_wrapped_sol_prefix_keys,
    utils::{mk_stake_pool_label, try_get_acc, try_token_acc_amt},
    TEMPORARY_JUP_AMM_LABEL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StakingCapCtrl {
    Rdy,
    NeedMsolLeg,
}

/// Notes:
/// - [`self.router.msol_leg_balance`] is only used to determine if
///   a SOL deposit will exceed marinade's staking cap, which has been
///   removed for a long time now, so we only fetch msol_leg account if
///   required ([`Self::has_staking_cap`])
#[derive(Debug, Clone)]
pub struct MarinadeSolAmm {
    pub cap: StakingCapCtrl,
    pub router: MarinadeRouterSol,
    pub stake_pool_label: String,

    // Cached PDAs below
    pub amm_key: [u8; 32],
    // sanctum router program fee token account PDAs
    pub msol_sr_fee_token_acc: [u8; 32],
}

const fn marinade_has_staking_cap(state: &State) -> bool {
    state.staking_sol_cap != u64::MAX
}

impl MarinadeSolAmm {
    pub const fn has_staking_cap(&self) -> bool {
        marinade_has_staking_cap(&self.router.state)
    }
}

impl Amm for MarinadeSolAmm {
    /// Initialize from main marinade state
    fn from_keyed_account(
        KeyedAccount {
            key,
            account,
            params,
        }: &KeyedAccount,
        _ctx: &AmmContext,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let state = State::borsh_de(account.data.as_slice())?;
        let cap = if marinade_has_staking_cap(&state) {
            StakingCapCtrl::NeedMsolLeg
        } else {
            StakingCapCtrl::Rdy
        };
        let (amm_key, _) =
            find_stake_pool_amm_key(key.as_array()).ok_or_else(|| invalid_pda("amm key"))?;
        let msol = MSOL_MINT_ADDR.into();
        let (msol_sr_fee_token_acc, _) = find_fee_token_account_pda(&MSOL_MINT_ADDR)
            .ok_or_else(|| invalid_pda(&format!("{} fee token account", msol)))?;

        let stake_pool_label = mk_stake_pool_label(&msol, params);

        Ok(Self {
            cap,
            router: MarinadeRouterSol {
                state,
                msol_leg_balance: 0,
            },
            stake_pool_label,
            amm_key,
            msol_sr_fee_token_acc,
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
        [NATIVE_MINT.into(), MSOL_MINT_ADDR.into()].into()
    }

    // marinade has their own SOL LP jup integration for SOL withdrawals
    fn unidirectional(&self) -> bool {
        true
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        let mut res = vec![STATE_PUBKEY.into()];
        if self.has_staking_cap() {
            res.push(LIQ_POOL_MSOL_LEG_PUBKEY.into());
        }
        res
    }

    fn update(&mut self, am: &AccountMap) -> anyhow::Result<()> {
        let state_acc = try_get_acc(am, &STATE_PUBKEY.into())?;
        let state = State::borsh_de(state_acc.data.as_slice())?;

        // mutate self first so that we always write
        // updated state even if we fail later on
        self.router.state = state;

        if self.has_staking_cap() {
            self.router.msol_leg_balance = try_token_acc_amt(am, &LIQ_POOL_MSOL_LEG_PUBKEY.into())?;
        }
        self.cap = StakingCapCtrl::Rdy;

        Ok(())
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
        if !matches!(self.cap, StakingCapCtrl::Rdy) {
            return Err(require_more_updates(1));
        }
        let [wsol, msol] = [NATIVE_MINT, MSOL_MINT_ADDR].map(Pubkey::from);
        if *input_mint == wsol && *output_mint == msol {
            Ok(conv_token_quote(
                msol,
                self.router
                    .deposit_sol_quoter()
                    .quote_deposit_sol(*amount)?,
            ))
        } else {
            Err(unsupported_mints(input_mint, output_mint))
        }
    }

    fn get_swap_and_account_metas(&self, sp: &SwapParams) -> anyhow::Result<SwapAndAccountMetas> {
        let SwapParams {
            source_mint,
            destination_mint,
            ..
        } = sp;
        let [wsol, msol] = [NATIVE_MINT, MSOL_MINT_ADDR].map(Pubkey::from);
        if *source_mint == wsol && *destination_mint == msol {
            let pre_keys = stake_wrapped_sol_prefix_keys(sp, &self.msol_sr_fee_token_acc);
            let pre = pre_keys
                .0
                .into_iter()
                .zip(STAKE_WRAPPED_SOL_PREFIX_IS_WRITER.0);

            let suf_accs = self.router.marinade_deposit_sol_suf_accs();
            let suf_keys = DepositSolSufAccs::suffix_accounts(&suf_accs);
            let suf_writable = DepositSolSufAccs::suffix_is_writable(&suf_accs);
            let suf = suf_keys.0.iter().zip(suf_writable.0);

            let mut account_metas = keys_writable_zipped_to_metas(pre.chain(suf));
            account_metas.push(sp.placeholder_account_meta());

            Ok(SwapAndAccountMetas {
                swap: Swap::StakeDexStakeWrappedSol,
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
        const STAKE_WSOL_LEN: usize =
            STAKE_WRAPPED_SOL_PREFIX_ACCS_LEN + MARINADE_DEPOSIT_SOL_IX_SUFFIX_ACCS_LEN;
        const LEN: usize = 1 + STAKE_WSOL_LEN;

        LEN
    }

    fn program_dependencies(&self) -> Vec<(Pubkey, String)> {
        vec![(
            MARINADE_STAKING_PROGRAM.into(),
            self.stake_pool_label.to_lowercase(),
        )]
    }
}
