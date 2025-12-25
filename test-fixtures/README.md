# test-fixtures

## Clone Program

```sh
solana program dump <addr> <programs/out.so>
```

Then, add `--upgradeable-program <addr> <programs/out.so> none` to the docker compose file to add it to the local validator.

## Clone Account

```sh
solana account --output json -o <account.json> <addr>
```

## Notes

### Cluster

Test cluster is assumed to be at epoch 1

### Programs

- All 3 deploys of SPL currently use the exact same binary, `programs/stake-pool.so`
- We use a BPF override of the stake program `programs/stake.so` instead of the mollusk 0.5 builtin for ease of modification

### Accounts

- All stake accounts are active with activationEpoch=0, deactivationEpoch=u64::MAX
- picosol vsa was reduced to 1k SOL stake so that we do not run into stake warmup limits (solana-test-validator starts out with 1M sol staked, so at most 90k SOL can be activated in epoch 0). This means vaidator stake is not consistent with pool/validator list state.
- lido's max stake validator at time of collection is of vote `8jxSHbS4qAnh5yueFp4D9ABXubKqMwXqF3HtdzQGuphp`
- bsol is the spl stake pool test-fixture of choice for depositing stake into because it pretty much contains all validators (lido's max validator, pico's validator, etc)
- `*-vault` accounts are the sanctum router program fee token accounts
