use std::collections::HashSet;

use anyhow::anyhow;
use generic_array_struct::generic_array_struct;
use jupiter_amm_interface::SwapParams;
use sanctum_router_std::{
    quote_prefund_swap_via_stake,
    sanctum_reserve_core::{FEE, POOL, POOL_SOL_RESERVES, PROTOCOL_FEE, UNSTAKE_PROGRAM},
    DepositStake, DepositStakeAddrs, DepositStakeIxAccsDestr, DepositStakeIxKeys,
    DepositStakeSufAccs, PrefundWithdrawStakePrefixAccsDestr, PrefundWithdrawStakePrefixKeys,
    WithdrawStake, WithdrawStakeSufAccs, DEPOSIT_STAKE_IX_IS_WRITER_NON_WSOL_OUT,
    DEPOSIT_STAKE_IX_IS_WRITER_WSOL_OUT, NATIVE_MINT, PREFUNDER,
    PREFUND_WITHDRAW_STAKE_PREFIX_IS_WRITER, STAKE_PROGRAM, SYSTEM_PROGRAM, SYSVAR_CLOCK,
};
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::{
    conv::keys_writable_zipped_to_metas,
    errs::{anyhow_unreachable, invalid_pda},
    pda::{
        jup::find_stake_pool_pair_amm_key,
        reserve::find_reserve_stake_account_record,
        router::{create_slumdog_stake_addr, find_bridge_stake_pda},
    },
    prefund_params,
    stake::traits::StakeRouter,
    ReserveRouterPpf,
};

pub fn prep_underlying_liquidities(
    a: &impl StakeRouter,
    b: &impl StakeRouter,
) -> Option<HashSet<Pubkey>> {
    match (a.underlying_liquidity(), b.underlying_liquidity()) {
        (None, None) => None,
        (a, b) => Some(a.into_iter().chain(b).map(Into::into).collect()),
    }
}

pub fn det_stake_pool_pair_amm_key(
    a: &impl StakeRouter,
    b: &impl StakeRouter,
) -> anyhow::Result<[u8; 32]> {
    find_stake_pool_pair_amm_key(a.main_state_key(), b.main_state_key())
        .map(|(a, _)| a)
        .ok_or_else(|| invalid_pda("pool pair amm key"))
}

#[generic_array_struct(destr pub)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefundWithdrawStakePreComp<T> {
    pub bridge_stake: T,
    pub slumdog_stake: T,
    pub slumdog_stake_acc_record: T,
    pub unstake_protocol_fee_dest: T,
}

pub type PrefundWithdrawStakePreCompKeys<'a> = PrefundWithdrawStakePreComp<&'a [u8; 32]>;

pub fn prefund_withdraw_stake_pre_keys<'a>(
    SwapParams {
        token_transfer_authority,
        source_token_account,
        source_mint,
        ..
    }: &'a SwapParams,
    comp_keys: &PrefundWithdrawStakePreCompKeys<'a>,
) -> PrefundWithdrawStakePrefixKeys<'a> {
    let PrefundWithdrawStakePreCompDestr {
        bridge_stake,
        slumdog_stake,
        slumdog_stake_acc_record,
        unstake_protocol_fee_dest,
    } = comp_keys.into_destr();
    PrefundWithdrawStakePrefixKeys::from_destr(PrefundWithdrawStakePrefixAccsDestr {
        user: token_transfer_authority.as_array(),
        inp_token: source_token_account.as_array(),
        bridge_stake,
        inp_mint: source_mint.as_array(),
        prefunder: &PREFUNDER,
        slumdog_stake,
        unstake_program: &UNSTAKE_PROGRAM,
        unstake_pool: &POOL,
        unstake_pool_sol_reserves: &POOL_SOL_RESERVES,
        unstake_fee: &FEE,
        slumdog_stake_acc_record,
        unstake_protocol_fee: &PROTOCOL_FEE,
        unstake_protocol_fee_dest,
        clock: &SYSVAR_CLOCK,
        stake_program: &STAKE_PROGRAM,
        system_program: &SYSTEM_PROGRAM,
    })
}

#[generic_array_struct(destr pub)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepositStakePreComp<T> {
    pub bridge_stake: T,
    pub out_fee_token: T,
}

pub type DepositStakePreCompKeys<'a> = DepositStakePreComp<&'a [u8; 32]>;

pub fn deposit_stake_pre_keys<'a>(
    SwapParams {
        token_transfer_authority,
        destination_mint,
        destination_token_account,
        ..
    }: &'a SwapParams,
    comp_keys: &DepositStakePreCompKeys<'a>,
    // TODO: rename to DepositStakeIxPreKeys upstream
) -> DepositStakeIxKeys<'a> {
    let DepositStakePreCompDestr {
        bridge_stake,
        out_fee_token,
    } = comp_keys.into_destr();
    DepositStakeIxKeys::from_destr(DepositStakeIxAccsDestr {
        user: token_transfer_authority.as_array(),
        inp_stake: bridge_stake,
        out_token: destination_token_account.as_array(),
        out_fee_token,
        out_mint: destination_mint.as_array(),
    })
}

pub fn try_prefund_swap_via_stake_metas(
    w: &impl WithdrawStake,
    d: &impl DepositStake,
    r: &ReserveRouterPpf,
    sp: &SwapParams,
    bridge_stake_seed: u32,
    deposit_sr_fee_token_acc: &[u8; 32],
) -> anyhow::Result<Vec<AccountMeta>> {
    let (rup, rf) = prefund_params(r);
    let vote = quote_prefund_swap_via_stake(
        w.withdraw_stake_val_quoters(),
        d.deposit_stake_quoter(),
        sp.in_amount,
        &rup,
        rf,
    )
    .map_err(|e| anyhow!("{e}"))?
    .1
    .inp
    .vote;

    let (bridge_stake, _) =
        find_bridge_stake_pda(sp.token_transfer_authority.as_array(), bridge_stake_seed)
            .ok_or_else(|| invalid_pda("bridge stake"))?;
    let slumdog_stake = create_slumdog_stake_addr(&bridge_stake);
    let (slumdog_stake_acc_record, _) = find_reserve_stake_account_record(&slumdog_stake)
        .ok_or_else(|| invalid_pda("slumdog stake acc record"))?;
    let unstake_protocol_fee_dest = &r.protocol_fee_dest;

    let w_pre_keys = prefund_withdraw_stake_pre_keys(
        sp,
        &PrefundWithdrawStakePreCompKeys::from_destr(PrefundWithdrawStakePreCompDestr {
            bridge_stake: &bridge_stake,
            slumdog_stake: &slumdog_stake,
            slumdog_stake_acc_record: &slumdog_stake_acc_record,
            unstake_protocol_fee_dest,
        }),
    );
    let w_pre_is_writer = PREFUND_WITHDRAW_STAKE_PREFIX_IS_WRITER;
    let w_pre = w_pre_keys.0.into_iter().zip(w_pre_is_writer.0);

    let w_suf_accs = w
        .withdraw_stake_suf_accs(&vote)
        // unreachable: quote just succeeded above so vote is supported
        .ok_or_else(anyhow_unreachable)?;
    let w_suf_keys = w_suf_accs.suffix_accounts();
    let w_suf_is_writable = w_suf_accs.suffix_is_writable();
    let w_suf = w_suf_keys
        .as_ref()
        .iter()
        .zip(w_suf_is_writable.as_ref().iter().copied());

    let d_pre_keys = deposit_stake_pre_keys(
        sp,
        &DepositStakePreCompKeys::from_destr(DepositStakePreCompDestr {
            bridge_stake: &bridge_stake,
            out_fee_token: deposit_sr_fee_token_acc,
        }),
    );
    let d_pre_is_writer = if sp.destination_mint.to_bytes() == NATIVE_MINT {
        DEPOSIT_STAKE_IX_IS_WRITER_WSOL_OUT
    } else {
        DEPOSIT_STAKE_IX_IS_WRITER_NON_WSOL_OUT
    };
    let d_pre = d_pre_keys.0.into_iter().zip(d_pre_is_writer.0);

    let d_suf_accs = d
        .deposit_stake_suf_accs(&DepositStakeAddrs {
            stake: bridge_stake,
            vote,
        })
        // unreachable: quote just succeeded above so vote is supported
        .ok_or_else(anyhow_unreachable)?;
    let d_suf_keys = d_suf_accs.suffix_accounts();
    let d_suf_is_writable = d_suf_accs.suffix_is_writable();
    let d_suf = d_suf_keys
        .as_ref()
        .iter()
        .zip(d_suf_is_writable.as_ref().iter().copied());

    let ph = sp.placeholder_account_meta();

    Ok(keys_writable_zipped_to_metas(
        w_pre
            .chain(w_suf)
            .chain(core::iter::once((ph.pubkey.as_array(), ph.is_writable)))
            .chain(d_pre)
            .chain(d_suf),
    ))
}
