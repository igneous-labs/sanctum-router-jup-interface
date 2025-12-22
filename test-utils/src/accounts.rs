use solana_account::Account;
use solana_pubkey::Pubkey;

use crate::CONST_PUBKEYS;

/// Clock with everything = 0
/// Currently only used as return value in get_accounts_to_update
pub fn mock_clock() -> Account {
    Account {
        data: vec![0; 40],
        owner: *CONST_PUBKEYS.sysvar_owner(),
        executable: false,
        lamports: 1169280,
        rent_epoch: u64::MAX,
    }
}

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
