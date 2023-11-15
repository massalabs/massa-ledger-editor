use std::{path::PathBuf, sync::Arc};

use parking_lot::RwLock;

use massa_db_exports::{DBBatch, MassaDBController, MassaIteratorMode, LEDGER_PREFIX};
use massa_final_state::FinalState;
use massa_ledger_editor::{
    get_db_config, get_final_state_config, get_ledger_config, get_mip_list, get_mip_stats_config,
    WrappedMassaDB,
};
use massa_ledger_exports::{KeyDeserializer};
use massa_ledger_worker::FinalLedger;
use massa_models::config::{T0, THREAD_COUNT};
use massa_models::slot::Slot;
use massa_models::{address::Address};
use massa_pos_exports::test_exports::MockSelectorController;
use massa_serialization::{DeserializeError, Deserializer};
use massa_time::MassaTime;
use massa_versioning::versioning::MipStore;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(name = "massa-ledger-editor")]
#[command(about = "Ledger editor", long_about = None)]
pub struct Cli {
    #[arg(
    short = 'p',
    long = "path",
    help = "Path of an existing db"
    )]
    path: PathBuf,
    #[arg(
    short = 'r',
    long = "initial_rolls_path",
    help = "Path of initial_rolls.json file (Massa node config file)"
    )]
    initial_rolls_path: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, PartialEq, Subcommand)]
enum Commands {
    #[command(about = "Convert ledger (from testnet 22 to testnet 23)")]
    ConvertLedger(ConvertLedgerArgs),
    #[command(about = "Edit ledger (Dummy impl for now)")]
    EditLedger,
    #[command(about = "Scan ledger (print trail hash + final state fingerprint")]
    ScanLedger,
    #[command(about = "Update MIP store (after a network shutdown) ensuring MIP info are coherent")]
    UpdateMipStore(UpdateMipStoreArgs),
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct ConvertLedgerArgs {
    #[arg(
    short = 'o',
    long = "output_path",
    help = "Path where to write converted db"
    )]
    output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct UpdateMipStoreArgs {
    #[arg(
    long = "shutdown_start",
    help = ""
    )]
    shutdown_start: u64,
    #[arg(
    long = "shutdown_end",
    help = ""
    )]
    shutdown_end: u64,
    #[arg(
    long = "genesis_timestamp",
    help = ""
    )]
    genesis_timestamp: u64,
}

const OLD_IDENT_BYTE_INDEX: usize = 34; // place of the ident byte

fn main() {
    let cli = Cli::parse();
    // println!("CLI args: {:?}", cli);

    // Init final state from disk
    let final_state = get_final_state(cli.clone());

    match cli.command {
        Commands::ConvertLedger(args_) => {
            convert_ledger(final_state, cli.initial_rolls_path, args_)
        }
        Commands::EditLedger => {
            edit_ledger(final_state);
        }
        Commands::ScanLedger => {
            scan_ledger(final_state);
        }
        Commands::UpdateMipStore(args_) => {
            update_mip_store(final_state, args_);
        }
    }
}

fn get_final_state(cli: Cli) -> Arc<RwLock<FinalState>> {

    let db_config = get_db_config(cli.path.clone());
    let ledger_config = get_ledger_config(cli.path.clone());
    let final_state_config =
        get_final_state_config(cli.path, Some(cli.initial_rolls_path.clone()));
    let mip_stats_config = get_mip_stats_config();

    let convert_ledger = matches!(cli.command, Commands::ConvertLedger(..));

    let wrapped_db = WrappedMassaDB::new(db_config, convert_ledger, false);
    let db = Arc::new(RwLock::new(
        Box::new(wrapped_db.0) as Box<(dyn MassaDBController + 'static)>
    ));

    let ledger = FinalLedger::new(ledger_config, db.clone());
    let mip_store =
        MipStore::try_from_db(db.clone(), mip_stats_config).expect("MIP store try_from_db failed");
    println!("mip store: {:?}", mip_store);

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

    final_state
}

fn convert_ledger(final_state: Arc<RwLock<FinalState>>, initial_rolls_path: PathBuf, args: ConvertLedgerArgs) {

    let new_db_config = get_db_config(args.output_path.clone());
    let new_ledger_config = get_ledger_config(args.output_path.clone());
    let new_final_state_config =
        get_final_state_config(args.output_path, Some(initial_rolls_path));
    let new_mip_stats_config = get_mip_stats_config();

    let new_wrapped_db = WrappedMassaDB::new(new_db_config, false, true);
    let new_db = Arc::new(RwLock::new(
        Box::new(new_wrapped_db.0) as Box<(dyn MassaDBController + 'static)>
    ));

    let db = final_state.read().db.clone();
    let new_ledger = FinalLedger::new(new_ledger_config, db.clone());
    let new_mip_store = MipStore::try_from((get_mip_list(), new_mip_stats_config))
        .expect("mip store creation failed");
    let (new_selector_controller, _new_selector_receiver) =
        MockSelectorController::new_with_receiver();
    let new_final_state = Arc::new(parking_lot::RwLock::new(
        FinalState::new(
            new_db.clone(),
            new_final_state_config,
            Box::new(new_ledger),
            new_selector_controller.clone(),
            new_mip_store,
            false,
        )
            .expect("could not init final state"),
    ));

    convert_from_testnet22_ledger_to_testnet23_ledger(final_state.clone(), new_final_state);
}

fn edit_old_value(
    old_serialized_key: &[u8],
    old_serialized_value: &[u8],
) -> (Vec<u8>, Vec<u8>, u8) {
    let mut new_serialized_key = Vec::new();
    new_serialized_key.extend_from_slice(LEDGER_PREFIX.as_bytes());
    new_serialized_key.extend_from_slice(&old_serialized_key[0..2]);

    // Add version byte
    new_serialized_key.push(0u8);

    new_serialized_key.extend_from_slice(&old_serialized_key[2..OLD_IDENT_BYTE_INDEX]);

    // Shift ident byte (to make room for version ident)
    let old_ident_value = old_serialized_key[OLD_IDENT_BYTE_INDEX];
    new_serialized_key.push(old_ident_value.saturating_add(1));
    if old_ident_value == 2 {
        new_serialized_key.extend_from_slice(&old_serialized_key[OLD_IDENT_BYTE_INDEX + 1..]);
    }

    (
        new_serialized_key,
        old_serialized_value.to_vec(),
        old_ident_value,
    )
}

fn add_version_ident_key(old_serialized_key: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut new_serialized_key_for_version_byte = Vec::new();

    new_serialized_key_for_version_byte.extend_from_slice(LEDGER_PREFIX.as_bytes());
    new_serialized_key_for_version_byte.extend_from_slice(&old_serialized_key[0..2]);
    // Add version byte
    new_serialized_key_for_version_byte.push(0u8);

    new_serialized_key_for_version_byte
        .extend_from_slice(&old_serialized_key[2..OLD_IDENT_BYTE_INDEX]);
    // Put the Version ident byte
    new_serialized_key_for_version_byte.push(0u8);

    (new_serialized_key_for_version_byte, vec![0u8])
}

#[allow(dead_code)]
#[allow(unused_variables)]
fn convert_from_testnet22_ledger_to_testnet23_ledger(
    old_final_state: Arc<RwLock<FinalState>>,
    new_final_state: Arc<RwLock<FinalState>>,
) {
    let old_db = old_final_state.read().db.clone();
    let new_db = new_final_state.read().db.clone();
    let old_ledger_cf = "ledger";

    let mut batches = Vec::new();
    let mut state_batch = DBBatch::new();
    let mut versioning_batch = DBBatch::new();
    let max_keys_per_batch = 10000;
    let mut cur_count = 0;
    let mut total_count = 0;
    let mut valid_count = 0;
    let mut sc_addr_count = 0;
    let mut user_addr_count = 0;
    let mut total_old_count = 0;

    for (old_serialized_key, old_serialized_value) in old_db
        .read()
        .iterator_cf(old_ledger_cf, MassaIteratorMode::Start)
    {
        total_old_count += 1;

        let (new_serialized_key, new_serialized_value, old_ident_value) =
            edit_old_value(&old_serialized_key, &old_serialized_value);
        let (new_serialized_key_for_version_byte, new_serialized_value_for_version_byte) =
            add_version_ident_key(&old_serialized_key);

        let new_kv_valid = new_final_state
            .read()
            .ledger
            .is_key_value_valid(&new_serialized_key, &new_serialized_value);
        let new_kv_for_version_byte_valid = new_final_state.read().ledger.is_key_value_valid(
            &new_serialized_key_for_version_byte,
            &new_serialized_value_for_version_byte,
        );

        if new_kv_valid && new_kv_for_version_byte_valid {
            let key_deser = KeyDeserializer::new(255, false);

            let (rest, key) = key_deser
                .deserialize::<DeserializeError>(&new_serialized_key)
                .unwrap();

            match key.address {
                Address::User(addr) => {
                    user_addr_count += 1;
                    if user_addr_count <= 5 {
                        println!("User address: {}", addr)
                    }
                }
                Address::SC(addr) => {
                    sc_addr_count += 1;
                    if sc_addr_count <= 5 {
                        println!("SC address: {}", addr)
                    }
                }
            }

            if cur_count >= max_keys_per_batch {
                batches.push((state_batch, versioning_batch));
                state_batch = DBBatch::new();
                versioning_batch = DBBatch::new();
                cur_count = 0;
            }

            new_db.read().put_or_update_entry_value(
                &mut state_batch,
                new_serialized_key,
                &new_serialized_value,
            );

            valid_count += 1;
            total_count += 1;
            cur_count += 1;

            if old_ident_value == 0 {
                new_db.read().put_or_update_entry_value(
                    &mut state_batch,
                    new_serialized_key_for_version_byte,
                    &new_serialized_value_for_version_byte,
                );

                valid_count += 1;
                total_count += 1;
                cur_count += 1;
            }
        } else {
            println!("Invalid key/value pair: {:?}", new_serialized_key);
            println!(
                "Invalid key/value pair: {:?}",
                new_serialized_key_for_version_byte
            );
        }
    }

    if cur_count > 0 {
        batches.push((state_batch, versioning_batch));
    }

    println!("Total OLD key/value count: {}", total_old_count);
    println!("Total key/value count: {}", total_count);
    println!("Valid key/value count: {}", valid_count);
    println!("UserAddress key/value count: {}", user_addr_count);
    println!("SCAddress key/value count: {}", sc_addr_count);

    let batches_len = batches.len();
    for (i, (state_batch, versioning_batch)) in batches.into_iter().enumerate() {
        new_final_state
            .write()
            .db
            .write()
            .write_batch(state_batch, versioning_batch, None);
        println!(
            "Batch {} of {} written (current key: {})",
            i + 1,
            batches_len,
            if i + 1 != batches_len {
                (i + 1) * max_keys_per_batch
            } else {
                total_count
            }
        );
    }
    new_final_state.write().db.write().flush().unwrap();
}

fn edit_ledger(final_state: Arc<RwLock<FinalState>>) {

    println!("Editing ledger...");

    final_state.write().init_execution_trail_hash();

    let db_valid = final_state.write().is_db_valid();
    println!("DB valid: {}", db_valid);
    /*// Here, we can create any state / versioning change we want
    let mut changes = LedgerChanges(PreHashMap::default());
    changes.0.insert(
        Address::from_str("AU12dhs6CsQk8AXFTYyUpc1P9e8GDf65ozU6RcigW68qfJV7vdbNf").unwrap(),
        SetUpdateOrDelete::Set(LedgerEntry {
            balance: Amount::from_mantissa_scale(100, 0).unwrap(),
            bytecode: Bytecode(Vec::new()),
            datastore: BTreeMap::default(),
        }),
    );*/

    // Apply the change to the batch
    /*final_state
        .write()
        .ledger
        .apply_changes_to_batch(changes, &mut state_batch);*/

    // Write the batch to the DB
    //db.write().write_batch(state_batch, versioning_batch, None);
}

fn scan_ledger(final_state: Arc<RwLock<FinalState>>) {
    // Here, we can query read functions from the state, but we could instead directly query the DB
    let get_execution_trail_hash = final_state.read().get_execution_trail_hash();
    println!("{:#?}", get_execution_trail_hash);

    let hash = final_state.read().get_fingerprint();
    println!("{:#?}", hash);
}

fn update_mip_store(final_state: Arc<RwLock<FinalState>>, args: UpdateMipStoreArgs) {

    let mut guard = final_state.write();
    let shutdown_start: Slot = Slot::new(args.shutdown_start, 0);
    let shutdown_end: Slot = Slot::new(args.shutdown_end, 0);
    /*
    let genesis_timestamp = match args.genesis_timestamp {
        Some(ts) => MassaTime::from_millis(ts),
        None => *GENESIS_TIMESTAMP,
    };
    */
    let genesis_timestamp = MassaTime::from_millis(args.genesis_timestamp);

    println!("Updating MIP store...");
    guard
        .mip_store
        .update_for_network_shutdown(
            shutdown_start,
            shutdown_end,
            THREAD_COUNT,
            T0,
            genesis_timestamp,
        )
        .expect("Cannot update MIP store");

    let mut db_batch = DBBatch::new();
    let mut db_versioning_batch = DBBatch::new();
    // Rewrite the whole MIP store
    guard
        .mip_store
        .update_batches(&mut db_batch, &mut db_versioning_batch, None)
        .expect("Cannot get batches in order to write MIP store");

    // Cleanup db (as we are going to rewrite all versioning entries)
    println!("Reset DB (versioning)...");
    // guard.mip_store.reset_db(db.clone());
    guard.mip_store.reset_db(final_state.read().db.clone());

    // Write updated entries
    println!("Writing MIP store...");
    guard
        .db
        .write()
        .write_batch(db_batch, db_versioning_batch, None);
    println!("Done.");

}