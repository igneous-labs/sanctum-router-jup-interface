use expect_test::expect;
use jupiter_amm_interface::{QuoteParams, SwapMode};
use sanctum_router_jup_interface::MarinadeSolAmm;
use sanctum_router_std::sanctum_marinade_liquid_staking_core::{self, MSOL_MINT_ADDR};
use solana_pubkey::Pubkey;
use test_utils::{ALL_FIXTURES, CONST_PUBKEYS, SVM};

use crate::common::{swap_test, SwapUserKeyedAccounts};

const STAKE_WRAPPED_SOL_NO_CAP_UPDATE_CYCLES: usize = 0;

const STAKE_WRAPPED_SOL_QUOTE_PARAMS: QuoteParams = QuoteParams {
    amount: 1_000_000_000,
    input_mint: *CONST_PUBKEYS.wsol_mint(),
    output_mint: Pubkey::new_from_array(MSOL_MINT_ADDR),
    swap_mode: SwapMode::ExactIn,
};

#[test]
fn sws_msol_fixture_basic() {
    expect![[r#"
        Quote {
            in_amount: 1000000000,
            out_amount: 776062653,
            fee_amount: 0,
            fee_mint: mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So,
            fee_pct: 0,
        }
    "#]]
    .assert_debug_eq(&SVM.with(|svm| {
        swap_test::<MarinadeSolAmm>(
            svm,
            &STAKE_WRAPPED_SOL_QUOTE_PARAMS,
            &ALL_FIXTURES,
            &sanctum_marinade_liquid_staking_core::STATE_PUBKEY.into(),
            SwapUserKeyedAccounts::from_qp(&STAKE_WRAPPED_SOL_QUOTE_PARAMS),
            STAKE_WRAPPED_SOL_NO_CAP_UPDATE_CYCLES,
        )
    }));
}
