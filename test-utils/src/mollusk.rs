use std::{collections::HashSet, path::Path};

use ahash::HashMap;
use mollusk_svm::result::{InstructionResult, ProgramResult};
use solana_account::Account;
use solana_instruction::{error::InstructionError, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{test_fixtures_dir, CONST_PUBKEYS, FIXTURE_PROGRAMS, TEST_EPOCH};

// Re-exports
pub use mollusk_svm::Mollusk;

thread_local! {
    pub static SVM: Mollusk = mollusk_base();
}

/// Successful execution result containing accounts after execution.
#[derive(Clone, Debug)]
pub struct ExecOk {
    pub resulting_accounts: HashMap<Pubkey, Account>,
    pub compute_units_consumed: u64,
    pub execution_time: u64,
    pub return_data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum ExecErr {
    Failure(ProgramError),
    UnknownError(InstructionError),
}

/// A mollusk instance with following programs all loaded in:
/// - all programs in test-fixtures/programs (NB: subdirs excluded)
/// - spl token program
/// - associated token program
/// - current epoch set to 1
pub fn mollusk_base() -> Mollusk {
    let mut svm = mollusk_with_token_progs();
    let paths = FIXTURE_PROGRAMS.into_iter().map(|(fname, key)| {
        (
            test_fixtures_dir()
                .join("programs")
                .join(fname)
                .with_extension("so"),
            key,
        )
    });
    mollusk_add_so_files(&mut svm, paths);

    svm.sysvars.clock.epoch = TEST_EPOCH;
    svm.sysvars.clock.slot = 432_001;

    svm
}

fn mollusk_with_token_progs() -> Mollusk {
    let mut res = Mollusk::default();
    mollusk_svm_programs_token::token::add_program(&mut res);
    mollusk_svm_programs_token::associated_token::add_program(&mut res);
    res
}

/// All programs have owner = BPF_LOADER_UPGRADEABLE
fn mollusk_add_so_files(
    svm: &mut Mollusk,
    so_files: impl IntoIterator<Item = (impl AsRef<Path>, Pubkey)>,
) {
    so_files.into_iter().for_each(|(path, key)| {
        svm.add_program_with_elf_and_loader(
            &key,
            &std::fs::read(path).unwrap(),
            CONST_PUBKEYS.bpf_loader_upgradeable(),
        );
    });
}

pub fn mollusk_exec(
    svm: &Mollusk,
    ixs: &[Instruction],
    onchain_state: &HashMap<Pubkey, Account>,
) -> Result<ExecOk, ExecErr> {
    let accs_bef = to_accs_vec(onchain_state, ixs);
    let res = svm.process_instruction_chain(ixs, &accs_bef);
    post_process(res)
}

fn to_accs_vec(am: &HashMap<Pubkey, Account>, ixs: &[Instruction]) -> Vec<(Pubkey, Account)> {
    let mut dedup = HashSet::new();
    ixs.iter()
        .flat_map(|ix| ix.accounts.iter().map(|a| a.pubkey))
        .filter(|&k| dedup.insert(k))
        .map(|k| {
            am.get(&k).map_or_else(
                // log missing pks here as required
                || (k, Default::default()),
                |v| (k, v.clone()),
            )
        })
        .collect()
}

fn post_process(
    InstructionResult {
        resulting_accounts,
        program_result,
        compute_units_consumed,
        execution_time,
        return_data,
        ..
    }: InstructionResult,
) -> Result<ExecOk, ExecErr> {
    match program_result {
        ProgramResult::Success => {
            let resulting_accounts = resulting_accounts.into_iter().collect();
            Ok(ExecOk {
                resulting_accounts,
                compute_units_consumed,
                execution_time,
                return_data,
            })
        }
        ProgramResult::Failure(e) => Err(ExecErr::Failure(e)),
        ProgramResult::UnknownError(e) => Err(ExecErr::UnknownError(e)),
    }
}
