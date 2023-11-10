use massa_db_exports::{DBBatch, MassaDBController};
use massa_final_state::FinalState;
use massa_ledger_editor::{
    get_db_config, get_final_state_config, get_ledger_config, get_mip_stats_config, WrappedMassaDB,
};
use massa_ledger_exports::{LedgerChanges, LedgerEntry, SetUpdateOrDelete};
use massa_ledger_worker::FinalLedger;
use massa_models::{address::Address, amount::Amount, bytecode::Bytecode, prehash::PreHashMap};
use massa_pos_exports::test_exports::MockSelectorController;
use massa_signature::KeyPair;
use massa_versioning::versioning::MipStore;
use parking_lot::RwLock;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Args {
    // Path to the ledger directory
    #[structopt(short, long)]
    path: PathBuf,

    /// Path to the initial rolls file
    #[structopt(short, long)]
    initial_rolls_path: PathBuf,

    /// Ledger size to reach, in GiB
    #[structopt(short, long)]
    target_ledger_size: Option<u64>,

    #[structopt(short, long, default_value = "205")]
    datastore_key_size: usize,

    #[structopt(long, default_value = "9999989")]
    datastore_value_size: usize,

    #[structopt(short, long, default_value = "9999989")]
    bytecode_size: usize,

    #[structopt(short, long)]
    single_problematic_value: bool,
}

fn calc_time_left(start: &Instant, done: usize, all: u64) -> Duration {
    let mut all_u32 = all;
    let telapsed = start.elapsed();
    let mut done_u32 = done;
    while all_u32 >= (u32::MAX as u64) {
        all_u32 /= 2;
        done_u32 /= 2;
    }
    let all_u32 = all_u32 as u32;
    let done_u32 = done_u32 as u32;
    if done_u32 == 0 {
        Duration::MAX
    } else {
        ((all_u32 * telapsed) / done_u32).saturating_sub(telapsed)
    }
}

#[inline]
fn generate_random_vector(size: usize, rng: &mut ThreadRng) -> Vec<u8> {
    (0..size).map(|_| rng.gen::<u8>()).collect::<Vec<u8>>()
}

fn create_ledger_entry(args: &Args, changes: &mut LedgerChanges, rng: &mut ThreadRng) -> usize {
    let mut sz = 0;
    let mut datastore = BTreeMap::default();

    let datastore_key = generate_random_vector(args.datastore_key_size, rng);
    sz += args.datastore_key_size;
    let datastore_val = generate_random_vector(args.datastore_value_size, rng);
    sz += args.datastore_value_size;
    let bytecode = Bytecode(generate_random_vector(args.bytecode_size, rng));
    sz += args.bytecode_size;

    let new_keypair = KeyPair::generate(0).expect("Unable to generate keypair");
    let new_pubkey = new_keypair.get_public_key();
    datastore.insert(datastore_key, datastore_val);
    changes.0.insert(
        Address::from_public_key(&new_pubkey),
        SetUpdateOrDelete::Set(LedgerEntry {
            balance: Amount::from_mantissa_scale(100, 0)
                .expect("Unable to get amount from mantissa scale"),
            bytecode,
            datastore,
        }),
    );

    sz
}

fn main() {
    let args = Args::from_args();

    // Retrieve config structures
    let db_config = get_db_config(args.path.clone());
    let ledger_config = get_ledger_config(args.path.clone());
    let final_state_config =
        get_final_state_config(args.path.clone(), Some(args.initial_rolls_path.clone()));
    let mip_stats_config = get_mip_stats_config();

    // Instantiate the main structs
    let wrapped_db = WrappedMassaDB::new(db_config, false, false);
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
    let mut slot = final_state
        .read()
        .db
        .read()
        .get_change_id()
        .expect("Unable to get change id");

    let mut rng = ThreadRng::default();
    // Edit section - Manual edits on the ledger or on the final_state
    if args.single_problematic_value {
        let mut changes = LedgerChanges(PreHashMap::default());
        let mut datastore = BTreeMap::default();
        let datastore_key = "TIM PROBLEMATIC KEY".as_bytes().to_vec();
        let datastore_val = generate_random_vector(args.datastore_value_size, &mut rng);
        let bytecode = Bytecode(generate_random_vector(args.bytecode_size, &mut rng));
        datastore.insert(datastore_key, datastore_val);
        changes.0.insert(
            Address::from_str("AU12hnuosRCREmeu6nQGvsG2EhHiEW9tzzwkpDabWwZHug1uFn2YS").unwrap(),
            SetUpdateOrDelete::Set(LedgerEntry {
                balance: Amount::from_mantissa_scale(100, 0)
                    .expect("Unable to get amount from mantissa scale"),
                bytecode,
                datastore,
            }),
        );
        let mut state_batch = DBBatch::new();
        {
            let mut final_state = final_state.write();
            final_state
                .ledger
                .apply_changes_to_batch(changes, &mut state_batch);
        }
        let versioning_batch = DBBatch::new();
        {
            let mut db = db.write();
            db.write_batch(state_batch, versioning_batch, Some(slot));
            db.flush().expect("Error while flushing DB");
        }
    } else {
        // let target: u64 = args.target_ledger_size.expect("Target ledger size not passed as argument") * 1024 * 1024 * 1024;
        let target: u64 = args.target_ledger_size.expect("Target ledger size not passed as argument") * 1024 * 1024;
        let mut added: usize = 0;
        println!("Filling the ledger with {target} bytes");
        let start = Instant::now();
        let batch_size: u64 = 1;
        let mut nwrite = 0;
        while added < target.try_into().unwrap() {
            let tleft = calc_time_left(&start, added, target);
            println!(
                "[{nwrite}] {:.2}MiB / {:.2}MiB done {:.5}% (ETA {:.2} mins){}",
                (added as f64) / (1024.0 * 1024.0),
                (target as f64) / (1024.0 * 1024.0),
                ((added as f64) / (target as f64)) * 100.0,
                tleft.as_secs_f64() / 60.0,
                " ".repeat(10),
            );
            let mut state_batch = DBBatch::new();
            let versioning_batch = DBBatch::new();
            // Here, we can create any state / versioning change we want
            let mut changes = LedgerChanges(PreHashMap::default());

            for _ in 0..batch_size {
                added += create_ledger_entry(&args, &mut changes, &mut rng);
            }

            // Apply the change to the batch
            {
                let mut final_state = final_state.write();
                final_state
                    .ledger
                    .apply_changes_to_batch(changes, &mut state_batch);
            }

            // Write the batch to the DB
            {
                let mut db = db.write();
                db.write_batch(state_batch, versioning_batch, Some(slot));
                nwrite += 1;
                if (nwrite % 20) == 0 {
                    db.flush().expect("Error while flushing DB");
                }
            }
            slot = slot.get_next_slot(32).expect("Unable to get next slot");
        }
        db.write().flush().expect("Error while flushing DB");
    }
}
