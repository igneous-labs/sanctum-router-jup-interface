use jupiter_amm_interface::{QuoteParams, SwapMode};
use sanctum_router_jup_interface::SplStakePoolSolAmm;
use test_utils::{mock_signer, mock_tokenkeg_acc, ALL_FIXTURES, CONST_PUBKEYS};

use crate::common::{
    swap_test, SwapUserAccsDestr, SwapUserKeyedAccounts, TEST_SIGNER, TOKEN_ACC_1, TOKEN_ACC_2,
};

const STAKE_WRAPPED_SOL_UPDATE_CYCLES: usize = 0;

const STAKE_WRAPPED_SOL_QUOTE_PARAMS: QuoteParams = QuoteParams {
    amount: 1_000_000_000,
    input_mint: *CONST_PUBKEYS.wsol_mint(),
    output_mint: *CONST_PUBKEYS.bsol_mint(),
    swap_mode: SwapMode::ExactIn,
};

#[test]
fn stake_wrapped_sol_bsol_fixture_basic() {
    swap_test::<SplStakePoolSolAmm>(
        &STAKE_WRAPPED_SOL_QUOTE_PARAMS,
        &ALL_FIXTURES,
        CONST_PUBKEYS.bsol_stake_pool(),
        SwapUserKeyedAccounts::from_destr(SwapUserAccsDestr {
            signer: (TEST_SIGNER, mock_signer()),
            inp_token_acc: (
                TOKEN_ACC_1,
                mock_tokenkeg_acc(
                    CONST_PUBKEYS.wsol_mint().to_bytes(),
                    TEST_SIGNER.to_bytes(),
                    STAKE_WRAPPED_SOL_QUOTE_PARAMS.amount,
                ),
            ),
            out_token_acc: (
                TOKEN_ACC_2,
                mock_tokenkeg_acc(
                    CONST_PUBKEYS.bsol_mint().to_bytes(),
                    TEST_SIGNER.to_bytes(),
                    0,
                ),
            ),
        }),
        STAKE_WRAPPED_SOL_UPDATE_CYCLES,
    );
}
