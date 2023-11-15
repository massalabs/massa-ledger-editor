# Massa ledger editor

## Make an edit

Ledger edits are made in `src/main.rs` `edit section`

## Run

If `massa-ledger-editor` was cloned at the same level as `massa` repository, you should use:

- Case 1: You want to manually edit/read the ledger

```commandline
cargo run -- -p ../massa/massa-node/storage/ledger/rocks_db -r ../massa/massa-node/base_config/initial_rolls.json scan-ledger
```

- Case 2: You want to convert the ledger from testnet22 to testnet23

```commandline
cargo run -- --path ../massa/massa-node/storage/testnet22_ledger/rocks_db/ -r ../massa/massa-node/base_config/initial_rolls.json convert-ledger --output-path ../massa/massa-node/storage/ledger/rocks_db/
```
