# Massa ledger editor

## Make an edit

Ledger edits are made in `src/main.rs` `edit section`

## Run

If `massa-ledger-editor` was cloned at the same level as `massa` repository, you should use:
- Case 1: You want to manually edit/read the ledger 
```
cargo run -- -p ../massa/massa-node/storage/ledger/rocks_db/
```
- Case 2: You want to convert the ledger from testnet22 to testnet23 
```
cargo run -- --path ../massa/massa-node/storage/testnet22_ledger/rocks_db/ --output-path ../massa/massa-node/storage/ledger/rocks_db/
```
