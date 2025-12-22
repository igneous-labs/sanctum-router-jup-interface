use generic_array_struct::generic_array_struct;
use solana_pubkey::Pubkey;

#[generic_array_struct(builder pub)]
pub struct ConstKeys<T> {
    // sysvars and cluster consts
    pub sysvar_owner: T,
    pub sysvar_clock: T,
    pub bpf_loader_upgradeable: T,
}

impl<T: Copy> ConstKeys<T> {
    #[inline]
    pub const fn memset(v: T) -> Self {
        Self([v; CONST_KEYS_LEN])
    }
}

pub const CONST_KEYS_STR: ConstKeys<&'static str> = ConstKeys::memset("")
    .const_with_sysvar_owner("Sysvar1111111111111111111111111111111111111")
    .const_with_sysvar_clock("SysvarC1ock11111111111111111111111111111111")
    .const_with_bpf_loader_upgradeable("BPFLoaderUpgradeab1e11111111111111111111111");

pub const CONST_PUBKEYS: ConstKeys<Pubkey> = {
    let mut res = ConstKeys::memset(Pubkey::new_from_array([0; 32]));
    let mut i = 0;
    while i < CONST_KEYS_LEN {
        res.0[i] = Pubkey::from_str_const(CONST_KEYS_STR.0[i]);
        i += 1;
    }
    res
};
