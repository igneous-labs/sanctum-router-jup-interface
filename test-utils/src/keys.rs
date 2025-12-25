use generic_array_struct::generic_array_struct;
use solana_pubkey::Pubkey;

// Re export consts from other crates
pub use mollusk_svm_programs_token::token::ID as TOKENKEG_PROGRAM;

/// const pubkeys that are otherwise not exported by other libs
#[generic_array_struct(pub)]
pub struct ConstKeys<T> {
    // sysvars and cluster consts
    pub sysvar_clock: T,
    pub bpf_loader_upgradeable: T,

    // programs
    pub stake_prog: T,
    pub spl_prog: T,
    pub sanctum_router_prog: T,
    pub jup_prog: T,

    pub wsol_mint: T,

    // fixtures
    pub bsol_mint: T,
    pub bsol_stake_pool: T,
}

impl<T: Copy> ConstKeys<T> {
    #[inline]
    pub const fn memset(v: T) -> Self {
        Self([v; _])
    }
}

pub const CONST_KEYS_STR: ConstKeys<&'static str> = ConstKeys::memset("")
    .const_with_sysvar_clock("SysvarC1ock11111111111111111111111111111111")
    .const_with_bpf_loader_upgradeable("BPFLoaderUpgradeab1e11111111111111111111111")
    .const_with_stake_prog("Stake11111111111111111111111111111111111111")
    .const_with_spl_prog("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")
    .const_with_sanctum_router_prog("stkitrT1Uoy18Dk1fTrgPw8W6MVzoCfYoAFT4MLsmhq")
    .const_with_jup_prog("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4")
    .const_with_wsol_mint("So11111111111111111111111111111111111111112")
    .const_with_bsol_mint("bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1")
    .const_with_bsol_stake_pool("stk9ApL5HeVAwPLr3TLhDXdZS8ptVu7zp6ov8HFDuMi");

pub const CONST_PUBKEYS: ConstKeys<Pubkey> = {
    let mut res = ConstKeys::memset(Pubkey::new_from_array([0; 32]));
    let mut i = 0;
    while i < CONST_KEYS_LEN {
        res.0[i] = Pubkey::from_str_const(CONST_KEYS_STR.0[i]);
        i += 1;
    }
    res
};
