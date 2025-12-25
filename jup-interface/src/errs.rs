use anyhow::anyhow;
use solana_pubkey::Pubkey;

pub(crate) fn acc_missing_err(pk: &Pubkey) -> anyhow::Error {
    anyhow!("{} missing in accounts_map", pk)
}

pub(crate) fn exact_out_unsupported_err() -> anyhow::Error {
    anyhow!("ExactOut SwapMode not supported")
}

pub(crate) fn unsupported_mints(inp: &Pubkey, out: &Pubkey) -> anyhow::Error {
    anyhow!("{inp} -> {out} swap not supported")
}

pub(crate) fn invalid_pda(name: &str) -> anyhow::Error {
    anyhow!("could not find PDA for {name}")
}

pub(crate) fn require_more_updates(n_updates_required: usize) -> anyhow::Error {
    anyhow!("require {n_updates_required} more update(s)")
}

pub(crate) fn invalid_data_err(pk: &Pubkey) -> anyhow::Error {
    anyhow!("invalid account data for {pk}")
}
