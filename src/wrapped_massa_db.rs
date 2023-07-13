use std::{collections::BTreeMap, sync::Arc};

use massa_db_exports::{MassaDBConfig, METADATA_CF, OPEN_ERROR, STATE_CF, VERSIONING_CF};
use massa_db_worker::MassaDB;
use massa_models::slot::{Slot, SlotDeserializer, SlotSerializer};
use parking_lot::Mutex;
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::ops::Bound::{Excluded, Included};

pub struct WrappedMassaDB(pub MassaDB);

impl WrappedMassaDB {
    /// Returns a new `MassaDB` instance
    pub fn new(config: MassaDBConfig, convert_ledger_from_old_format: bool, create_if_missing: bool) -> Self {
        let mut db_opts = Options::default();

        // Note: no need to create anything (it can even be misleading if we specify the wrong path)
        if create_if_missing {
            db_opts.create_if_missing(true);
            db_opts.create_missing_column_families(true);
        }

        let db = if convert_ledger_from_old_format {
            DB::open_cf_descriptors(
                &db_opts,
                &config.path,
                vec![
                    ColumnFamilyDescriptor::new(STATE_CF, Options::default()),
                    ColumnFamilyDescriptor::new("ledger", Options::default()),
                    ColumnFamilyDescriptor::new(METADATA_CF, Options::default()),
                    ColumnFamilyDescriptor::new(VERSIONING_CF, Options::default()),
                ],
            )
            .expect(OPEN_ERROR)
        } else {
            DB::open_cf_descriptors(
                &db_opts,
                &config.path,
                vec![
                    ColumnFamilyDescriptor::new(STATE_CF, Options::default()),
                    ColumnFamilyDescriptor::new(METADATA_CF, Options::default()),
                    ColumnFamilyDescriptor::new(VERSIONING_CF, Options::default()),
                ],
            )
            .expect(OPEN_ERROR)
        };

        let db = Arc::new(db);
        let current_batch = Arc::new(Mutex::new(WriteBatch::default()));

        let change_id_deserializer = SlotDeserializer::new(
            (Included(u64::MIN), Included(u64::MAX)),
            (Included(0), Excluded(config.thread_count)),
        );

        let massa_db = MassaDB {
            db,
            config,
            change_history: BTreeMap::new(),
            change_history_versioning: BTreeMap::new(),
            change_id_serializer: SlotSerializer::new(),
            change_id_deserializer,
            current_batch,
        };

        if massa_db.get_change_id().is_err() {
            massa_db.set_initial_change_id(Slot {
                period: 0,
                thread: 0,
            });
        }

        WrappedMassaDB(massa_db)
    }
}
