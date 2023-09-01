use massa_db_exports::{DBBatch, MassaDBController};
use massa_final_state::FinalState;
use massa_ledger_editor::{
    get_db_config, get_final_state_config, get_ledger_config, get_mip_stats_config, WrappedMassaDB,
};
use massa_ledger_exports::{LedgerChanges, LedgerEntry, SetUpdateOrDelete};
use massa_ledger_worker::FinalLedger;
use massa_models::{address::Address, amount::Amount, bytecode::Bytecode, prehash::PreHashMap};
use massa_pos_exports::test_exports::MockSelectorController;
use massa_versioning::versioning::MipStore;
use parking_lot::RwLock;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::{Instant, Duration}};
use structopt::StructOpt;
use massa_signature::KeyPair;

#[derive(Debug, StructOpt)]
pub struct Args {
    // path is used to open an existing db
    #[structopt(short, long)]
    path: PathBuf,
    #[structopt(short, long)]
    initial_rolls_path: PathBuf,
}

fn calc_time_left(start: &Instant, done: u64, all: u64) -> Duration {
    let mut all_u32 = all;
    let telapsed = start.elapsed();
    let mut done_u32 = done;
    while all_u32 >= (u32::MAX as u64) {
        all_u32 /= 2;
        done_u32 /=2;
    }
    let all_u32 = all_u32 as u32;
    let done_u32 = done_u32 as u32;
    if done_u32 == 0 {
        Duration::MAX
    } else {
        ((all_u32 * telapsed) / done_u32).saturating_sub(telapsed)
    }
}

fn main() {
    let args = Args::from_args();

    // Set up the following flags depending on what we want to do.
    let convert_ledger = false;
    let edit_ledger = true;

    // Retrieve config structures
    let db_config = get_db_config(args.path.clone());
    let ledger_config = get_ledger_config(args.path.clone());
    let final_state_config = get_final_state_config(args.path, Some(args.initial_rolls_path));
    let mip_stats_config = get_mip_stats_config();

    // Instantiate the main structs
    let wrapped_db = WrappedMassaDB::new(db_config, convert_ledger, false);
    let db = Arc::new(RwLock::new(
        Box::new(wrapped_db.0) as Box<(dyn MassaDBController + 'static)>
    ));

    let ledger = FinalLedger::new(ledger_config, db.clone());
    let mip_store =
        MipStore::try_from_db(db.clone(), mip_stats_config).expect("MIP store try_from_db failed");

    let (selector_controller, _selector_receiver) = MockSelectorController::new_with_receiver();
    let final_state = Arc::new(parking_lot::RwLock::new(
        FinalState::new(
            db.clone(),
            final_state_config,
            Box::new(ledger),
            selector_controller.clone(),
            mip_store,
            false,
        )
        .expect("could not init final state"),
    ));

    // Edit section - Manual edits on the ledger or on the final_state
    if edit_ledger {
        let target: u64 = 700 * 1024 * 1024 * 1024;
        let mut added = 0;
        println!("Filling the ledger with {target} bytes");
        let start = Instant::now();
        let batch_size: u64 = 10;
        while added < target {
            let tleft = calc_time_left(&start, added, target);
            println!("{added}/{target} done {:.5}% (ETA {tleft:?})", ((added as f64) / (target as f64)) * 100.0);
            let mut state_batch = DBBatch::new();
            let versioning_batch = DBBatch::new();
            // Here, we can create any state / versioning change we want
            let mut changes = LedgerChanges(PreHashMap::default());

            for n in 0..batch_size {
                let mut datastore = BTreeMap::default();
                let new_keypair = KeyPair::generate(0).unwrap();
                let new_pubkey = new_keypair.get_public_key();
                let key_size = 255 - n.to_be_bytes().len() - added.to_be_bytes().len();
                let mut datastore_key = Vec::from(added.to_be_bytes());
                datastore_key.extend((n+1).to_be_bytes());
                datastore_key.extend(vec![0; key_size]);
                datastore.insert(datastore_key, vec![99; 9_999_999]);
                changes.0.insert(
                    Address::from_public_key(&new_pubkey),
                    SetUpdateOrDelete::Set(LedgerEntry {
                        balance: Amount::from_mantissa_scale(100, 0).unwrap(),
                        bytecode: Bytecode(vec![42; 9_999_999]),
                        datastore,
                    }),
                );
            }

            // Apply the change to the batch
            final_state
                .write()
                .ledger
                .apply_changes_to_batch(changes, &mut state_batch);

            // Write the batch to the DB
            db.write().write_batch(state_batch, versioning_batch, None);
            added += 9_999_999 + 254 + 9_999_999;
        }
    }
}
