use jupiter_amm_interface::SwapParams;
use sanctum_router_std::{
    solido_legacy_core::TOKENKEG_PROGRAM, StakeWrappedSolPrefixAccsDestr,
    StakeWrappedSolPrefixKeys, WithdrawWrappedSolPrefixAccsDestr, WithdrawWrappedSolPrefixKeys,
    SOL_BRIDGE_OUT, SYSTEM_PROGRAM, WSOL_BRIDGE_IN,
};

pub fn stake_wrapped_sol_prefix_keys<'a>(
    SwapParams {
        token_transfer_authority,
        source_token_account,
        destination_token_account,
        destination_mint,
        source_mint,
        ..
    }: &'a SwapParams,
    out_fee_token: &'a [u8; 32],
) -> StakeWrappedSolPrefixKeys<'a> {
    StakeWrappedSolPrefixKeys::from_destr(StakeWrappedSolPrefixAccsDestr {
        user: token_transfer_authority.as_array(),
        inp_wsol: source_token_account.as_array(),
        out_token: destination_token_account.as_array(),
        wsol_bridge_in: &WSOL_BRIDGE_IN,
        sol_bridge_out: &SOL_BRIDGE_OUT,
        out_fee_token,
        out_mint: destination_mint.as_array(),
        wsol_mint: source_mint.as_array(),
        token_program: &TOKENKEG_PROGRAM,
        system_program: &SYSTEM_PROGRAM,
    })
}

pub fn withdraw_wrapped_sol_prefix_keys<'a>(
    SwapParams {
        token_transfer_authority,
        source_token_account,
        destination_token_account,
        destination_mint,
        source_mint,
        ..
    }: &'a SwapParams,
    wsol_fee_token: &'a [u8; 32],
) -> WithdrawWrappedSolPrefixKeys<'a> {
    WithdrawWrappedSolPrefixKeys::from_destr(WithdrawWrappedSolPrefixAccsDestr {
        user: token_transfer_authority.as_array(),
        wsol_mint: destination_mint.as_array(),
        token_program: &TOKENKEG_PROGRAM,
        inp_token: source_token_account.as_array(),
        out_wsol: destination_token_account.as_array(),
        wsol_fee_token,
        inp_mint: source_mint.as_array(),
    })
}
