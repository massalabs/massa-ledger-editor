use massa_ledger_exports::{
    LedgerChanges, LedgerConfig, LedgerController, LedgerEntry, SetUpdateOrDelete,
};
use massa_ledger_worker::FinalLedger;
use massa_models::{
    address::Address,
    amount::Amount,
    bytecode::Bytecode,
    config::{MAX_DATASTORE_KEY_LENGTH, MAX_DATASTORE_VALUE_LENGTH, THREAD_COUNT},
    prehash::PreHashMap,
    slot::Slot,
};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(short, long)]
    path: PathBuf,
}

fn main() {
    // mirror massa ledger setup
    let args = Args::from_args();
    let config = LedgerConfig {
        thread_count: THREAD_COUNT,
        disk_ledger_path: args.path,
        initial_ledger_path: PathBuf::new(),
        max_key_length: MAX_DATASTORE_KEY_LENGTH,
        max_ledger_part_size: 0,
        max_datastore_value_length: MAX_DATASTORE_VALUE_LENGTH,
    };
    let mut ledger = FinalLedger::new(config);

    // edit section
    let mut changes = LedgerChanges(PreHashMap::default());
    changes.0.insert(
        Address::from_str("AU12dhs6CsQk8AXFTYyUpc1P9e8GDf65ozU6RcigW68qfJV7vdbNf").unwrap(),
        SetUpdateOrDelete::Set(LedgerEntry {
            balance: Amount::from_mantissa_scale(100, 0),
            bytecode: Bytecode(Vec::new()),
            datastore: BTreeMap::default(),
        }),
    );
    ledger.apply_changes(changes, Slot::min());

    // scan section
    let balance = ledger.get_balance(
        &Address::from_str("AU12AXf3ngq3e5zPWikXu3QGR18JtX3wbRuduA3126joowvPwVWdk").unwrap(),
    );
    println!("{:#?}", balance);
}
