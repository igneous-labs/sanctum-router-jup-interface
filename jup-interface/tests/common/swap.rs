use anyhow::anyhow;
use generic_array_struct::generic_array_struct;
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas,
    SwapMode, SwapParams,
};
use sanctum_router_std::{
    DepositStakeIxData, PrefundWithdrawStakeIxData, StakeWrappedSolIxData, WithdrawWrappedSolIxData,
};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use test_utils::{mock_signer, mock_tokenkeg_acc, mollusk_exec, ExecOk, Mollusk, CONST_PUBKEYS};

use crate::common::{AMM_CONTEXT, TEST_SIGNER, TOKEN_ACC_1, TOKEN_ACC_2};

#[generic_array_struct(destr pub)]
#[derive(Default)]
#[repr(transparent)]
pub struct SwapUserAccs<T> {
    pub signer: T,
    pub inp_token_acc: T,
    pub out_token_acc: T,
}

pub type SwapUserKeyedAccounts = SwapUserAccs<(Pubkey, Account)>;

impl SwapUserKeyedAccounts {
    pub fn from_qp(
        QuoteParams {
            input_mint,
            output_mint,
            amount,
            swap_mode,
        }: &QuoteParams,
    ) -> Self {
        assert!(matches!(swap_mode, SwapMode::ExactIn));
        Self::from_destr(SwapUserAccsDestr {
            signer: (TEST_SIGNER, mock_signer()),
            inp_token_acc: (
                TOKEN_ACC_1,
                mock_tokenkeg_acc(input_mint.to_bytes(), TEST_SIGNER.to_bytes(), *amount),
            ),
            out_token_acc: (
                TOKEN_ACC_2,
                mock_tokenkeg_acc(output_mint.to_bytes(), TEST_SIGNER.to_bytes(), 0),
            ),
        })
    }
}

/// The whole point of it all:
///
/// - inits Amm struct
/// - runs n update cycles
/// - quote
/// - swap
/// - mollusk execute swap
/// - assert amount in and out matches quote
///
/// Returns quoted [`Quote`] if test passes
pub fn swap_test<A: Amm>(
    svm: &Mollusk,
    qp: &QuoteParams,
    onchain_state: &AccountMap,
    init_pk: &Pubkey,
    user: SwapUserKeyedAccounts,
    // number of update cycles before this Amm
    // is ready for use
    n_update_cycles: usize,
) -> Quote {
    let mut amm: A = init_amm(onchain_state, &AMM_CONTEXT, init_pk);

    (0..n_update_cycles).for_each(|_| update_cycle(&mut amm, onchain_state).unwrap());

    let quote = amm.quote(qp).unwrap();
    let saam = amm
        .get_swap_and_account_metas(&SwapParams {
            swap_mode: qp.swap_mode,
            in_amount: quote.in_amount,
            out_amount: quote.out_amount,
            source_mint: qp.input_mint,
            destination_mint: qp.output_mint,
            source_token_account: user.inp_token_acc().0,
            destination_token_account: user.out_token_acc().0,
            token_transfer_authority: user.signer().0,
            // dont-cares
            quote_mint_to_referrer: Default::default(),
            jupiter_program_id: CONST_PUBKEYS.jup_prog(),
            missing_dynamic_accounts_as_default: Default::default(),
        })
        .unwrap();
    let ixs = saam_to_ixs(qp.amount, saam);

    let user_keys = SwapUserAccs(user.0.each_ref().map(|(pk, _)| *pk));

    let accs_bef = onchain_state
        .iter()
        .map(|(pk, ac)| (*pk, ac.clone()))
        // need to include user accounts into state as well
        .chain(user.0)
        // sysvars that are read by instructions from accounts
        // need to be explicitly added as accounts
        .chain([
            svm.sysvars.keyed_account_for_stake_history_sysvar(),
            svm.sysvars.keyed_account_for_clock_sysvar(),
        ])
        .collect();

    let ExecOk {
        resulting_accounts: accs_aft,
        ..
    } = mollusk_exec(svm, &ixs, &accs_bef).unwrap();

    assert_balance_change(
        &accs_bef,
        &accs_aft,
        user_keys.inp_token_acc(),
        // NB: negative
        -i128::from(quote.in_amount),
    );
    assert_balance_change(
        &accs_bef,
        &accs_aft,
        user_keys.out_token_acc(),
        quote.out_amount.into(),
    );

    quote
}

pub fn init_amm<A: Amm>(am: &AccountMap, ctx: &AmmContext, init_pk: &Pubkey) -> A {
    let (key, account) = am.get_key_value(init_pk).unwrap();
    A::from_keyed_account(
        &KeyedAccount {
            key: *key,
            account: account.clone(),
            params: None,
        },
        ctx,
    )
    .unwrap()
}

pub fn update_cycle(amm: &mut impl Amm, onchain_state: &AccountMap) -> anyhow::Result<()> {
    let accs = amm.get_accounts_to_update();

    accs.iter().try_for_each(|a| {
        onchain_state
            .contains_key(a)
            .then_some(())
            .ok_or_else(|| anyhow!("Missing acc {a}"))
    })?;

    amm.update(onchain_state)
}

fn rem_jup_placeholder(accounts: &mut Vec<AccountMeta>) {
    accounts.retain(|a| a.pubkey != *CONST_PUBKEYS.jup_prog());
}

fn user_to_signer(accounts: &mut [AccountMeta]) {
    // All sanctum router prog swap instructions have
    // signer as the first account
    let user_pk = accounts[0].pubkey;
    accounts.iter_mut().for_each(|a| {
        if a.pubkey == user_pk {
            a.is_signer = true;
        }
    });
}

fn saam_to_ixs(
    amt: u64,
    SwapAndAccountMetas {
        swap,
        account_metas: mut accounts,
    }: SwapAndAccountMetas,
) -> Vec<Instruction> {
    match swap {
        Swap::StakeDexStakeWrappedSol => {
            let data = StakeWrappedSolIxData::new(amt).to_buf().into();
            rem_jup_placeholder(&mut accounts);
            user_to_signer(&mut accounts);
            vec![Instruction {
                program_id: sanctum_router_std::SANCTUM_ROUTER_PROGRAM.into(),
                accounts,
                data,
            }]
        }
        Swap::StakeDexWithdrawWrappedSol => {
            let data = WithdrawWrappedSolIxData::new(amt).to_buf().into();
            rem_jup_placeholder(&mut accounts);
            user_to_signer(&mut accounts);
            vec![Instruction {
                program_id: sanctum_router_std::SANCTUM_ROUTER_PROGRAM.into(),
                accounts,
                data,
            }]
        }
        Swap::StakeDexPrefundWithdrawStakeAndDepositStake { bridge_stake_seed } => {
            let sep_idx = accounts
                .iter()
                .position(|a| a.pubkey == *CONST_PUBKEYS.jup_prog())
                .unwrap();
            user_to_signer(&mut accounts);
            let pws = &accounts[..sep_idx];
            let ds = &accounts[sep_idx + 1..];
            vec![
                Instruction {
                    program_id: sanctum_router_std::SANCTUM_ROUTER_PROGRAM.into(),
                    accounts: pws.into(),
                    data: PrefundWithdrawStakeIxData::new(amt, bridge_stake_seed)
                        .to_buf()
                        .into(),
                },
                Instruction {
                    program_id: sanctum_router_std::SANCTUM_ROUTER_PROGRAM.into(),
                    accounts: ds.into(),
                    data: DepositStakeIxData::new().to_buf().into(),
                },
            ]
        }
        _ => unreachable!(),
    }
}

fn assert_balance_change(
    accs_bef: &AccountMap,
    accs_aft: &AccountMap,
    pk: &Pubkey,
    expected_change: i128,
) {
    let [balance_bef, balance_aft] = [accs_bef, accs_aft].map(|am| {
        let acc = am.get(pk).unwrap();
        i128::from(balance_from_token_acc_data(&acc.data).unwrap())
    });
    assert_eq!(balance_aft - balance_bef, expected_change);
}

fn balance_from_token_acc_data(token_acc_data: &[u8]) -> Option<u64> {
    u64_le_at(token_acc_data, 64)
}

fn u64_le_at(data: &[u8], at: usize) -> Option<u64> {
    chunk_at(data, at).map(|c| u64::from_le_bytes(*c))
}

fn chunk_at<const N: usize>(data: &[u8], at: usize) -> Option<&[u8; N]> {
    data.get(at..).and_then(|s| s.first_chunk())
}
