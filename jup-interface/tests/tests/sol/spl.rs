use expect_test::expect;
use jupiter_amm_interface::{Amm, QuoteParams, SwapMode};
use sanctum_router_jup_interface::SplStakePoolSolAmm;
use test_utils::{ALL_FIXTURES, CONST_PUBKEYS, SVM};

use crate::common::{init_amm, swap_test, SwapUserKeyedAccounts, AMM_CONTEXT};

const STAKE_WRAPPED_SOL_UPDATE_CYCLES: usize = 0;

const WITHDRAW_WRAPPED_SOL_UPDATE_CYCLES: usize = 1;

const STAKE_WRAPPED_SOL_QUOTE_PARAMS: QuoteParams = QuoteParams {
    amount: 1_000_000_000,
    input_mint: *CONST_PUBKEYS.wsol_mint(),
    output_mint: *CONST_PUBKEYS.bsol_mint(),
    swap_mode: SwapMode::ExactIn,
};

const WITHDRAW_WRAPPED_SOL_QUOTE_PARAMS: QuoteParams = QuoteParams {
    input_mint: STAKE_WRAPPED_SOL_QUOTE_PARAMS.output_mint,
    output_mint: STAKE_WRAPPED_SOL_QUOTE_PARAMS.input_mint,
    ..STAKE_WRAPPED_SOL_QUOTE_PARAMS
};

#[test]
fn stake_wrapped_sol_bsol_fixture_basic() {
    expect![[r#"
        Quote {
            in_amount: 1000000000,
            out_amount: 816729365,
            fee_amount: 0,
            fee_mint: bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1,
            fee_pct: 0,
        }
    "#]]
    .assert_debug_eq(&SVM.with(|svm| {
        swap_test::<SplStakePoolSolAmm>(
            svm,
            &STAKE_WRAPPED_SOL_QUOTE_PARAMS,
            &ALL_FIXTURES,
            CONST_PUBKEYS.bsol_stake_pool(),
            SwapUserKeyedAccounts::from_qp(&STAKE_WRAPPED_SOL_QUOTE_PARAMS),
            STAKE_WRAPPED_SOL_UPDATE_CYCLES,
        )
    }));
}

#[test]
fn withdraw_wrapped_sol_bsol_fixture_basic() {
    expect![[r#"
        Quote {
            in_amount: 1000000000,
            out_amount: 1223049081,
            fee_amount: 1122317,
            fee_mint: So11111111111111111111111111111111111111112,
            fee_pct: 0.000916797273513819,
        }
    "#]]
    .assert_debug_eq(&SVM.with(|svm| {
        swap_test::<SplStakePoolSolAmm>(
            svm,
            &WITHDRAW_WRAPPED_SOL_QUOTE_PARAMS,
            &ALL_FIXTURES,
            CONST_PUBKEYS.bsol_stake_pool(),
            SwapUserKeyedAccounts::from_qp(&WITHDRAW_WRAPPED_SOL_QUOTE_PARAMS),
            WITHDRAW_WRAPPED_SOL_UPDATE_CYCLES,
        )
    }));
}

#[test]
fn verify_withdraw_wrapped_sol_update_cycles() {
    let amm: SplStakePoolSolAmm =
        init_amm(&ALL_FIXTURES, &AMM_CONTEXT, CONST_PUBKEYS.bsol_stake_pool());
    expect!["require 1 more update(s)"].assert_eq(
        &amm.quote(&WITHDRAW_WRAPPED_SOL_QUOTE_PARAMS)
            .unwrap_err()
            .to_string(),
    );
}
