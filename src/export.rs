use std::{collections::BTreeMap, ops::Bound::Included, sync::Arc};

use massa_db_exports::{LEDGER_PREFIX, STATE_CF};
use massa_final_state::FinalState;
use massa_ledger_exports::{KeyDeserializer, KeyType, LedgerEntry};
use massa_models::{
    amount::{Amount, AmountDeserializer},
    bytecode::{Bytecode, BytecodeDeserializer},
    datastore::Datastore,
};
use massa_serialization::{DeserializeError, Deserializer};
use parking_lot::RwLock;

use crate::args::ExportLedgerArgs;

pub fn export_ledger(final_state: Arc<RwLock<FinalState>>, args: ExportLedgerArgs) {
    let ledger = &final_state
        .read()
        .db
        .read()
        .prefix_iterator_cf(STATE_CF, LEDGER_PREFIX.as_bytes())
        .take_while(|(key, _)| key.starts_with(LEDGER_PREFIX.as_bytes()))
        .collect::<Vec<_>>();

    let mut new_ledger: BTreeMap<String, LedgerEntry> = BTreeMap::new();
    let mut error_count = 0;

    let mut datastore_entries: BTreeMap<String, Vec<(Vec<u8>, Vec<u8>)>> = BTreeMap::new();

    for (key, db_entry) in ledger.iter() {
        let key_deser = KeyDeserializer::new(255, false);
        let (_rest, key) = match key_deser.deserialize::<DeserializeError>(key) {
            Ok(result) => result,
            Err(e) => {
                println!("Key deserialization error: {}", e);
                error_count += 1;
                continue;
            }
        };
        let address = key.address;

        let entry = new_ledger
            .entry(address.to_string())
            .or_insert_with(|| LedgerEntry {
                balance: Amount::from_mantissa_scale(0, 0).unwrap(),
                bytecode: Bytecode(Vec::new()),
                datastore: Datastore::default(),
            });

        match key.key_type {
            KeyType::BALANCE => {
                match AmountDeserializer::new(Included(Amount::MIN), Included(Amount::MAX))
                    .deserialize::<DeserializeError>(db_entry)
                {
                    Ok((_rest, balance)) => {
                        entry.balance = balance;
                    }
                    Err(e) => {
                        println!(
                            "Balance deserialization error for address {}: {}",
                            address, e
                        );
                        error_count += 1;
                    }
                }
            }
            KeyType::BYTECODE => {
                match BytecodeDeserializer::new(10_000_000)
                    .deserialize::<DeserializeError>(db_entry)
                {
                    Ok((_rest, bytecode)) => {
                        entry.bytecode = bytecode;
                    }
                    Err(e) => {
                        println!(
                            "Bytecode deserialization error for address {}: {}",
                            address, e
                        );
                        error_count += 1;
                    }
                }
            }
            KeyType::DATASTORE(datastore_vec) => {
                datastore_entries
                    .entry(address.to_string())
                    .or_default()
                    .push((datastore_vec, db_entry.to_vec()));
            }
            KeyType::VERSION => {
                // ignored
            }
        }
    }

    for (address, entries) in datastore_entries {
        if let Some(entry) = new_ledger.get_mut(&address) {
            for (key, value) in entries {
                entry.datastore.insert(key, value);
            }
        }
    }

    let json = match serde_json::to_string_pretty(&new_ledger) {
        Ok(json) => json,
        Err(e) => {
            println!("JSON serialization error: {}", e);
            return;
        }
    };

    match std::fs::write(&args.output_path, json) {
        Ok(_) => {
            println!(
                "Ledger successfully exported to {} ({} errors)",
                args.output_path.display(),
                error_count
            );
        }
        Err(e) => {
            println!("Error writing JSON file: {}", e);
        }
    }
}
