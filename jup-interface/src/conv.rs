//! Note on rust_decimal conversion:
//! - Decimal::from_f64() returns None if infinite or NaN (before_fees = 0)
//! - All fee_pct calculations return Decimal::zero on error instead of failing
//!   in order to not block core functionality and to preserve same behaviour as original lib

use std::borrow::Borrow;

use jupiter_amm_interface::Quote;
use rust_decimal::{
    prelude::{FromPrimitive, Zero},
    Decimal,
};
use sanctum_router_std::{
    sanctum_u64_ratio::{Floor, Ratio},
    DepositStakeQuote, Prefund, TokenQuote, WithRouterFee, WithdrawStakeQuote,
};
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

pub(crate) fn conv_token_quote(
    out_mint: Pubkey,
    TokenQuote {
        inp: in_amount,
        out: out_amount,
        fee: fee_amount,
    }: TokenQuote,
) -> Quote {
    let fee_pct =
        Decimal::from_f64((fee_amount as f64) / (out_amount.saturating_add(fee_amount) as f64))
            .unwrap_or_else(Decimal::zero);
    Quote {
        in_amount,
        out_amount,
        fee_amount,
        fee_mint: out_mint,
        fee_pct,
    }
}

pub(crate) fn conv_withdraw_sol_quote(
    out_mint: Pubkey,
    WithRouterFee { router_fee, quote }: WithRouterFee<TokenQuote>,
) -> Quote {
    conv_token_quote(
        out_mint,
        TokenQuote {
            fee: router_fee.saturating_add(quote.fee),
            ..quote
        },
    )
}

pub(crate) fn conv_prefund_swap_via_stake_quote(
    out_mint: Pubkey,
    (w, d): (Prefund<WithdrawStakeQuote>, DepositStakeQuote),
) -> Quote {
    // total fees is sum of following fees in sequence:
    // 1. withdraw_from's withdraw stake fees (input mint)
    // 2. instant unstake fee for slumdog stake to repay prefund (SOL)
    // 3. deposit_to's deposit stake fees (output mint)
    // 4. stakedex's global fees (output mint)

    let d = d.with_router_fee();
    let out_fees = d.quote.fee.saturating_add(d.router_fee);
    let out_total_bef_fees = d.quote.out.saturating_add(out_fees);

    // To convert from input mint & SOL to output mint,
    // we approximate the tokens' relative exchange rates by
    // using the ratios between the same amount of value

    let sol_over_inp = Floor(Ratio {
        n: w.quote.out.lamports.total().saturating_add(w.prefund_fee),
        d: w.quote.inp.saturating_add(w.quote.fee),
    });
    let out_over_sol = Floor(Ratio {
        n: out_total_bef_fees,
        d: d.quote.inp.lamports.total(),
    });

    let prefund_fee = out_over_sol.apply(w.prefund_fee).unwrap_or_default();
    let withdraw_stake_fee = sol_over_inp
        .apply(w.quote.fee)
        .and_then(|f| out_over_sol.apply(f))
        .unwrap_or_default();

    conv_token_quote(
        out_mint,
        TokenQuote {
            fee: withdraw_stake_fee
                .saturating_add(prefund_fee)
                .saturating_add(out_fees),
            inp: w.quote.inp,
            out: d.quote.out,
        },
    )
}

pub(crate) fn keys_writable_zipped_to_metas(
    zipped: impl Iterator<Item = (impl Borrow<[u8; 32]>, impl Borrow<bool>)>,
) -> Vec<AccountMeta> {
    zipped
        .map(|(key, writable)| AccountMeta {
            pubkey: Pubkey::new_from_array(*key.borrow()),
            is_signer: false, // The signer is elevated by the jupiter instruction, otherwise uses shared accounts and elevated internally before CPI
            is_writable: *writable.borrow(),
        })
        .collect()
}
