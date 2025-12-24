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
use sanctum_router_std::{TokenQuote, WithRouterFee};
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
