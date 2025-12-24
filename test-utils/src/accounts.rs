use sanctum_spl_token_core::state::account::RawTokenAccount;
use solana_account::Account;
use solana_pubkey::Pubkey;

use crate::{CONST_PUBKEYS, TOKENKEG_PROGRAM};

/// Creates a mock program account with given `programdata_address`
pub fn mock_prog_acc(programdata_address: Pubkey) -> Account {
    let mut data = vec![0u8; 36];
    // UpgradeableLoaderState::Program discriminant, is bincode enum
    data[0] = 2;
    data[4..].copy_from_slice(programdata_address.as_array());
    Account {
        data,
        owner: *CONST_PUBKEYS.bpf_loader_upgradeable(),
        executable: true,
        // dont-cares
        lamports: 1_000_000_000,
        rent_epoch: u64::MAX,
    }
}

const TOKEN_ACC_RENT_EXEMPTION: u64 = 2_039_280;

// TODO: these should probably be in `sanctum_spl_token_core`
const COPTION_NONE: [u8; 4] = [0; 4];
const COPTION_SOME: [u8; 4] = [1, 0, 0, 0];

fn mock_token_acc_with_prog(a: RawTokenAccount, token_prog: Pubkey) -> Account {
    let lamports = match a.native_rent_exemption_coption_discm {
        COPTION_NONE => TOKEN_ACC_RENT_EXEMPTION,
        COPTION_SOME => [a.amount, a.native_rent_exemption]
            .map(u64::from_le_bytes)
            .iter()
            .sum(),
        _err => unreachable!(),
    };
    Account {
        lamports,
        data: a.as_acc_data_arr().into(),
        owner: token_prog,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

fn mock_tokenkeg_acc_full(a: RawTokenAccount) -> Account {
    mock_token_acc_with_prog(a, TOKENKEG_PROGRAM)
}

/// Adapted from
/// https://github.com/igneous-labs/sanctum-solana-utils/blob/dc8426210a11e2c74ff21ae272dee953d457d0cd/sanctum-solana-test-utils/src/token/tokenkeg.rs#L44-L84
fn raw_token_acc(mint: [u8; 32], auth: [u8; 32], amt: u64) -> RawTokenAccount {
    let (native_rent_exemption_coption_discm, native_rent_exemption) =
        if mint == CONST_PUBKEYS.wsol_mint().to_bytes() {
            (COPTION_SOME, TOKEN_ACC_RENT_EXEMPTION.to_le_bytes())
        } else {
            (COPTION_NONE, [0; 8])
        };
    RawTokenAccount {
        mint,
        auth,
        amount: amt.to_le_bytes(),
        delegate_coption_discm: [0; 4],
        delegate: [0; 32],
        state: 1u8,
        native_rent_exemption_coption_discm,
        native_rent_exemption,
        delegated_amount: [0; 8],
        close_auth_coption_discm: [0; 4],
        close_auth: [0; 32],
    }
}

pub fn mock_tokenkeg_acc(mint: [u8; 32], auth: [u8; 32], amt: u64) -> Account {
    mock_tokenkeg_acc_full(raw_token_acc(mint, auth, amt))
}

pub fn mock_signer() -> Account {
    Account {
        lamports: 1_000_000_000,
        rent_epoch: u64::MAX,
        ..Default::default()
    }
}
