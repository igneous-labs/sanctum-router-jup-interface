use jupiter_amm_interface::AccountMap;
use solana_account::Account;
use solana_pubkey::Pubkey;

use crate::errs::{acc_missing_err, invalid_data_err};

pub(crate) fn try_get_acc<'a>(am: &'a AccountMap, addr: &Pubkey) -> anyhow::Result<&'a Account> {
    am.get(addr).ok_or_else(|| acc_missing_err(addr))
}

pub(crate) fn try_token_acc_amt(am: &AccountMap, addr: &Pubkey) -> anyhow::Result<u64> {
    let d = &try_get_acc(am, addr)?.data;
    Ok(u64::from_le_bytes(
        *d.get(..72)
            .and_then(|s| s.last_chunk())
            .ok_or_else(|| invalid_data_err(addr))?,
    ))
}

pub(crate) fn mk_stake_pool_label(mint: &Pubkey, params: &Option<serde_json::Value>) -> String {
    params
        .as_ref()
        .map_or_else(|| None, |v| v.as_str())
        .map_or_else(
            || format!("{} stake pool", mint),
            |token_name| format!("{token_name} stake pool"),
        )
}
