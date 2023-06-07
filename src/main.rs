use massa_db_exports::{
    DBBatch, MassaDBController, MassaIteratorMode, LEDGER_PREFIX,
};
use massa_final_state::FinalState;
use massa_ledger_editor::{
    get_db_config, get_final_state_config, get_ledger_config, get_mip_stats_config, WrappedMassaDB,
};
use massa_ledger_exports::{LedgerChanges, LedgerEntry, SetUpdateOrDelete};
use massa_ledger_worker::FinalLedger;
use massa_models::{address::Address, amount::Amount, bytecode::Bytecode, prehash::PreHashMap};
use massa_pos_exports::test_exports::MockSelectorController;
use massa_versioning::{mips::MIP_LIST, versioning::MipStore};
use parking_lot::RwLock;
use std::{collections::BTreeMap, path::PathBuf, str::FromStr, sync::Arc};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Args {
    // path is used to open an existing db
    #[structopt(short, long)]
    path: PathBuf,
    // output_path is used to create a new db (only when using convert_from_testnet22_ledger_to_testnet23_ledger)
    #[structopt(short, long)]
    output_path: PathBuf,
}

#[allow(dead_code)]
#[allow(unused_variables)]
fn convert_from_testnet22_ledger_to_testnet23_ledger(old_final_state: Arc<RwLock<FinalState>>, new_final_state: Arc<RwLock<FinalState>>) {

    let old_db = old_final_state.read().db.clone();
    let new_db = new_final_state.read().db.clone();

    let old_ledger_cf = "ledger";

    let mut state_batch = DBBatch::new();
    let versioning_batch = DBBatch::new();

    for (old_serialized_key, old_serialized_value) in old_db
        .read()
        .iterator_cf(old_ledger_cf, MassaIteratorMode::Start)
    {
        let mut new_serialized_key = Vec::new();
        new_serialized_key.extend_from_slice(LEDGER_PREFIX.as_bytes());
        new_serialized_key.extend_from_slice(&old_serialized_key);

        new_db.read().put_or_update_entry_value(
            &mut state_batch,
            new_serialized_key,
            &old_serialized_value,
        );
    }

    new_final_state.write().db.write().write_batch(state_batch, versioning_batch, None);
}

fn main() {
    let args = Args::from_args();

    // Set up the following flags depending on what we want to do.
    let convert_ledger = true;
    let edit_ledger = false;
    let scan_ledger = false;

    // Retrieve config structures
    let db_config = get_db_config(args.path.clone());
    let ledger_config = get_ledger_config(args.path.clone());
    let final_state_config = get_final_state_config(args.path);
    let mip_stats_config = get_mip_stats_config();

    // Instantiate the main structs
    let wrapped_db = WrappedMassaDB::new(db_config, convert_ledger);
    let db = Arc::new(RwLock::new(
        Box::new(wrapped_db.0) as Box<(dyn MassaDBController + 'static)>
    ));

    let ledger = FinalLedger::new(ledger_config, db.clone());
    let mip_store =
        MipStore::try_from((MIP_LIST, mip_stats_config)).expect("mip store creation failed");
    let (selector_controller, _selector_receiver) = MockSelectorController::new_with_receiver();
    let final_state = Arc::new(parking_lot::RwLock::new(
        FinalState::new(
            db.clone(),
            final_state_config,
            Box::new(ledger),
            selector_controller.clone(),
            mip_store,
            true,
        )
        .expect("could not init final state"),
    ));

    // Edit section - Conversion from testnet22 to testnet23 ledger
    if convert_ledger {
        let new_db_config = get_db_config(args.output_path.clone());
        let new_ledger_config = get_ledger_config(args.output_path.clone());
        let new_final_state_config = get_final_state_config(args.output_path);
        let new_mip_stats_config = get_mip_stats_config();

        let new_wrapped_db = WrappedMassaDB::new(new_db_config, false);
        let new_db = Arc::new(RwLock::new(
            Box::new(new_wrapped_db.0) as Box<(dyn MassaDBController + 'static)>
        ));
    
        let new_ledger = FinalLedger::new(new_ledger_config, db.clone());
        let new_mip_store =
            MipStore::try_from((MIP_LIST, new_mip_stats_config)).expect("mip store creation failed");
        let (new_selector_controller, _new_selector_receiver) = MockSelectorController::new_with_receiver();
        let new_final_state = Arc::new(parking_lot::RwLock::new(
            FinalState::new(
                new_db.clone(),
                new_final_state_config,
                Box::new(new_ledger),
                new_selector_controller.clone(),
                new_mip_store,
                true,
            )
            .expect("could not init final state"),
        ));

        convert_from_testnet22_ledger_to_testnet23_ledger(final_state.clone(), new_final_state.clone());
    }

    // Edit section - Manual edits on the ledger or on the final_state
    if edit_ledger {
        let mut state_batch = DBBatch::new();
        let versioning_batch = DBBatch::new();

        // Here, we can create any state / versioning change we want
        let mut changes = LedgerChanges(PreHashMap::default());
        changes.0.insert(
            Address::from_str("AU12dhs6CsQk8AXFTYyUpc1P9e8GDf65ozU6RcigW68qfJV7vdbNf").unwrap(),
            SetUpdateOrDelete::Set(LedgerEntry {
                balance: Amount::from_mantissa_scale(100, 0),
                bytecode: Bytecode(Vec::new()),
                datastore: BTreeMap::default(),
            }),
        );

        // Apply the change to the batch
        final_state
            .write()
            .ledger
            .apply_changes_to_batch(changes, &mut state_batch);

        // Write the batch to the DB
        db.write().write_batch(state_batch, versioning_batch, None);
    }

    // Scan section
    if scan_ledger {
        // Here, we can query read functions from the state, but we could instead directly query the DB
        let balance = final_state.read().ledger.get_balance(
            &Address::from_str("AU12AXf3ngq3e5zPWikXu3QGR18JtX3wbRuduA3126joowvPwVWdk").unwrap(),
        );
        println!("{:#?}", balance);
    }
}
