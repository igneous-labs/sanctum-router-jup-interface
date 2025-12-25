use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas,
    SwapMode, SwapParams,
};
use sanctum_router_std::{
    sanctum_spl_stake_pool_core::StakePool, DepositSol, DepositSolQuoter, DepositSolSufAccs,
    SplDepositSolQuoter, SplRouterDepositSol, SplRouterSol, SplSolSufAccs, SplWithdrawSolQuoter,
    WithdrawSol, WithdrawSolQuoter, WithdrawSolSufAccs, NATIVE_MINT, SANCTUM_ROUTER_PROGRAM,
    SPL_DEPOSIT_SOL_IX_SUFFIX_ACCS_LEN, SPL_WITHDRAW_SOL_IX_SUFFIX_ACCS_LEN,
    STAKE_WRAPPED_SOL_PREFIX_ACCS_LEN, STAKE_WRAPPED_SOL_PREFIX_IS_WRITER,
    WITHDRAW_WRAPPED_SOL_PREFIX_ACCS_LEN, WITHDRAW_WRAPPED_SOL_PREFIX_IS_WRITER,
};
use solana_pubkey::Pubkey;

use crate::{
    conv::{conv_token_quote, conv_withdraw_sol_quote, keys_writable_zipped_to_metas},
    errs::{exact_out_unsupported_err, invalid_pda, require_more_updates, unsupported_mints},
    pda::{
        jup::find_stake_pool_amm_key, router::find_fee_token_account_pda,
        spl::find_withdraw_auth_pda,
    },
    sol::common::{stake_wrapped_sol_prefix_keys, withdraw_wrapped_sol_prefix_keys},
    utils::{mk_stake_pool_label, try_get_acc},
    TEMPORARY_JUP_AMM_LABEL,
};

#[derive(Debug, Clone)]
pub struct SplStakePoolSolAmm {
    pub curr_epoch: Arc<AtomicU64>,
    pub router: SplStakePoolSolState,
    pub stake_pool_label: String,

    // Cached PDAs below
    pub amm_key: [u8; 32],
    // sanctum router program fee token account PDAs
    pub pool_mint_sr_fee_token_acc: [u8; 32],
    pub wsol_sr_fee_token_acc: [u8; 32],
}

macro_rules! cmn_mthd {
    ($state:expr, $mthd:ident) => {
        match $state {
            SplStakePoolSolState::Full(s) => s.$mthd(),
            SplStakePoolSolState::Init(s) => s.$mthd(),
        }
    };
}

impl SplStakePoolSolAmm {
    fn load_curr_epoch(&self) -> u64 {
        self.curr_epoch.load(Ordering::Relaxed)
    }

    pub fn deposit_sol_quoter(&self) -> SplDepositSolQuoter<'_> {
        let base = cmn_mthd!(&self.router, deposit_sol_quoter);
        SplDepositSolQuoter {
            curr_epoch: self.load_curr_epoch(),
            ..base
        }
    }

    pub fn try_withdraw_sol_quoter(&self) -> anyhow::Result<SplWithdrawSolQuoter<'_>> {
        let base = match &self.router {
            SplStakePoolSolState::Init(_) => return Err(require_more_updates(1)),
            SplStakePoolSolState::Full(s) => s.withdraw_sol_quoter(),
        };
        Ok(SplWithdrawSolQuoter {
            curr_epoch: self.load_curr_epoch(),
            ..base
        })
    }

    pub const fn spl_sol_suf_accs(&self) -> SplSolSufAccs<'_> {
        cmn_mthd!(&self.router, spl_sol_suf_accs)
    }
}

/// Notes:
/// - the `curr_epoch` fields of the inner structs are not used,
///   the shared [`SplStakePoolSolAmm::curr_epoch`] is patched in
///   at quoting time instead
#[derive(Debug, Clone, PartialEq)]
pub enum SplStakePoolSolState {
    /// On init, only StakePool account is available.
    /// Can only quote StakeWrappedSol in this state
    Init(SplRouterDepositSol),

    /// Transitions to this state upon first [`Amm::update`]
    /// which fetches reserves account.
    /// Can quote both StakeWrappedSol and WithdrawWrappedSol
    /// in this state
    Full(SplRouterSol),
}

macro_rules! cmn_field {
    ($field:ident, $T:ty) => {
        pub const fn $field(&self) -> $T {
            match self {
                Self::Init(SplRouterDepositSol { $field, .. })
                | Self::Full(SplRouterSol { $field, .. }) => $field,
            }
        }
    };
}

impl SplStakePoolSolState {
    cmn_field!(stake_pool_program, &[u8; 32]);
    cmn_field!(stake_pool_addr, &[u8; 32]);
    cmn_field!(withdraw_authority_program_address, &[u8; 32]);
    cmn_field!(stake_pool, &StakePool);
}

impl Amm for SplStakePoolSolAmm {
    /// Initialize from stake pool main account
    fn from_keyed_account(
        KeyedAccount {
            key,
            account,
            params,
        }: &KeyedAccount,
        amm_context: &AmmContext,
    ) -> anyhow::Result<Self> {
        let [stake_pool_program, stake_pool_addr] = [account.owner, *key].map(Pubkey::to_bytes);
        let curr_epoch = amm_context.clock_ref.epoch.clone();
        let (withdraw_authority_program_address, _) =
            find_withdraw_auth_pda(&stake_pool_program, &stake_pool_addr)
                .ok_or_else(|| invalid_pda("withdraw auth"))?;
        let stake_pool = StakePool::borsh_de(account.data.as_slice())?;

        let (amm_key, _) =
            find_stake_pool_amm_key(key.as_array()).ok_or_else(|| invalid_pda("amm key"))?;

        let [p, w] = [&stake_pool.pool_mint, &NATIVE_MINT].map(|mint| {
            find_fee_token_account_pda(mint)
                .ok_or_else(|| invalid_pda(&format!("{} fee token account", Pubkey::from(*mint))))
        });
        let pool_mint_sr_fee_token_acc = p?.0;
        let wsol_sr_fee_token_acc = w?.0;

        let stake_pool_label = mk_stake_pool_label(&stake_pool.pool_mint.into(), params);

        Ok(Self {
            curr_epoch,
            amm_key,
            pool_mint_sr_fee_token_acc,
            wsol_sr_fee_token_acc,
            stake_pool_label,
            router: SplStakePoolSolState::Init(SplRouterDepositSol {
                stake_pool_program,
                stake_pool_addr,
                withdraw_authority_program_address,
                stake_pool,
                // dummy val, unused
                curr_epoch: 0,
            }),
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
        [
            NATIVE_MINT.into(),
            self.router.stake_pool().pool_mint.into(),
        ]
        .into()
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        [
            (*self.router.stake_pool_addr()).into(),
            self.router.stake_pool().reserve_stake.into(),
        ]
        .into()
    }

    fn update(&mut self, am: &AccountMap) -> anyhow::Result<()> {
        let [sp, rsv] = [
            *self.router.stake_pool_addr(),
            self.router.stake_pool().reserve_stake,
        ]
        .map(|addr| {
            let addr = addr.into();
            try_get_acc(am, &addr)
        });
        let sp = sp?;
        let rsv = rsv?;

        let stake_pool = StakePool::borsh_de(sp.data.as_slice())?;
        let reserve_stake_lamports = rsv.lamports;

        let new_state = SplStakePoolSolState::Full(SplRouterSol {
            stake_pool_program: *self.router.stake_pool_program(),
            stake_pool_addr: *self.router.stake_pool_addr(),
            withdraw_authority_program_address: *self.router.withdraw_authority_program_address(),
            stake_pool,
            reserve_stake_lamports,
            curr_epoch: 0,
        });

        self.router = new_state;

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
        let [wsol, pool_mint] = [NATIVE_MINT, self.router.stake_pool().pool_mint].map(Pubkey::from);
        if *input_mint == wsol && *output_mint == pool_mint {
            Ok(conv_token_quote(
                pool_mint,
                self.deposit_sol_quoter().quote_deposit_sol(*amount)?,
            ))
        } else if *input_mint == pool_mint && *output_mint == wsol {
            Ok(conv_withdraw_sol_quote(
                wsol,
                self.try_withdraw_sol_quoter()?
                    .quote_withdraw_sol(*amount)?
                    .withdraw_sol_with_router_fee(),
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
        let [wsol, pool_mint] = [NATIVE_MINT, self.router.stake_pool().pool_mint].map(Pubkey::from);
        if *source_mint == wsol && *destination_mint == pool_mint {
            let pre_keys = stake_wrapped_sol_prefix_keys(sp, &self.pool_mint_sr_fee_token_acc);
            let pre = pre_keys
                .0
                .into_iter()
                .zip(STAKE_WRAPPED_SOL_PREFIX_IS_WRITER.0);

            let suf_accs = self.spl_sol_suf_accs();
            let suf_keys = DepositSolSufAccs::suffix_accounts(&suf_accs);
            let suf_writable = DepositSolSufAccs::suffix_is_writable(&suf_accs);
            let suf = suf_keys.0.iter().zip(suf_writable.0);

            let mut account_metas = keys_writable_zipped_to_metas(pre.chain(suf));
            account_metas.push(sp.placeholder_account_meta());

            Ok(SwapAndAccountMetas {
                swap: Swap::StakeDexStakeWrappedSol,
                account_metas,
            })
        } else if *source_mint == pool_mint && *destination_mint == wsol {
            let pre_keys = withdraw_wrapped_sol_prefix_keys(sp, &self.wsol_sr_fee_token_acc);
            let pre = pre_keys
                .0
                .into_iter()
                .zip(WITHDRAW_WRAPPED_SOL_PREFIX_IS_WRITER.0);

            let suf_accs = self.spl_sol_suf_accs();
            let suf_keys = WithdrawSolSufAccs::suffix_accounts(&suf_accs);
            let suf_writable = WithdrawSolSufAccs::suffix_is_writable(&suf_accs);
            let suf = suf_keys.0.iter().zip(suf_writable.0);

            let mut account_metas = keys_writable_zipped_to_metas(pre.chain(suf));
            account_metas.push(sp.placeholder_account_meta());

            Ok(SwapAndAccountMetas {
                swap: Swap::StakeDexWithdrawWrappedSol,
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
            STAKE_WRAPPED_SOL_PREFIX_ACCS_LEN + SPL_DEPOSIT_SOL_IX_SUFFIX_ACCS_LEN;
        const WITHDRAW_WSOL_LEN: usize =
            WITHDRAW_WRAPPED_SOL_PREFIX_ACCS_LEN + SPL_WITHDRAW_SOL_IX_SUFFIX_ACCS_LEN;
        const MAX_LEN: usize = 1 + if STAKE_WSOL_LEN > WITHDRAW_WSOL_LEN {
            STAKE_WSOL_LEN
        } else {
            WITHDRAW_WSOL_LEN
        };

        MAX_LEN
    }

    fn program_dependencies(&self) -> Vec<(Pubkey, String)> {
        vec![(
            (*self.router.stake_pool_program()).into(),
            self.stake_pool_label.to_lowercase(),
        )]
    }
}
