use expect_test::expect;
use jupiter_amm_interface::{QuoteParams, SwapMode};
use sanctum_router_jup_interface::LidoReserveStakeAmm;
use sanctum_router_std::{solido_legacy_core::STSOL_MINT_ADDR, NATIVE_MINT};
use solana_pubkey::Pubkey;
use test_utils::{ALL_FIXTURES, SVM};

use crate::common::{swap_test, SwapUserKeyedAccounts, LIDO_RESERVE_STAKE_UPDATE_CYCLES};

const TO_SOL_QUOTE_PARAMS: QuoteParams = QuoteParams {
    amount: 1_000_000_000,
    input_mint: Pubkey::new_from_array(STSOL_MINT_ADDR),
    output_mint: Pubkey::new_from_array(NATIVE_MINT),
    swap_mode: SwapMode::ExactIn,
};

#[test]
fn psvs_stsol_wsol_fixture_basic() {
    expect![[r#"
        Quote {
            in_amount: 1000000000,
            out_amount: 1211175670,
            fee_amount: 3512851,
            fee_mint: So11111111111111111111111111111111111111112,
            fee_pct: 0.00289197678192268,
        }
    "#]]
    .assert_debug_eq(&SVM.with(|svm| {
        swap_test::<LidoReserveStakeAmm>(
            svm,
            &TO_SOL_QUOTE_PARAMS,
            &ALL_FIXTURES,
            &Pubkey::default(),
            SwapUserKeyedAccounts::from_qp(&TO_SOL_QUOTE_PARAMS),
            LIDO_RESERVE_STAKE_UPDATE_CYCLES,
        )
    }));
}
