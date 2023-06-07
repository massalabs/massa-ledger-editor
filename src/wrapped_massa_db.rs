use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use lsmtree::SparseMerkleTree;
use massa_db_exports::{
    MassaDBConfig, CF_ERROR, CRUD_ERROR, LSMTREE_NODES_CF, LSMTREE_VALUES_CF, METADATA_CF,
    OPEN_ERROR, STATE_CF, STATE_HASH_KEY, VERSIONING_CF,
};
use massa_db_worker::{MassaDB, MassaDbLsmtree};
use massa_models::slot::{Slot, SlotDeserializer, SlotSerializer};
use parking_lot::{Mutex, RwLock};
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::ops::Bound::{Excluded, Included};

pub struct WrappedMassaDB(pub MassaDB);

impl WrappedMassaDB {
    /// Returns a new `MassaDB` instance
    pub fn new(config: MassaDBConfig, convert_ledger_from_old_format: bool) -> Self {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let db = if convert_ledger_from_old_format {
            DB::open_cf_descriptors(
                &db_opts,
                &config.path,
                vec![
                    ColumnFamilyDescriptor::new(STATE_CF, Options::default()),
                    ColumnFamilyDescriptor::new("ledger", Options::default()),
                    ColumnFamilyDescriptor::new(METADATA_CF, Options::default()),
                    ColumnFamilyDescriptor::new(LSMTREE_NODES_CF, Options::default()),
                    ColumnFamilyDescriptor::new(LSMTREE_VALUES_CF, Options::default()),
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
                    ColumnFamilyDescriptor::new(LSMTREE_NODES_CF, Options::default()),
                    ColumnFamilyDescriptor::new(LSMTREE_VALUES_CF, Options::default()),
                    ColumnFamilyDescriptor::new(VERSIONING_CF, Options::default()),
                ],
            )
            .expect(OPEN_ERROR)
        };

        let db = Arc::new(db);

        let current_batch = Arc::new(Mutex::new(WriteBatch::default()));
        let current_hashmap = Arc::new(RwLock::new(HashMap::new()));

        let change_id_deserializer = SlotDeserializer::new(
            (Included(u64::MIN), Included(u64::MAX)),
            (Included(0), Excluded(config.thread_count)),
        );

        let nodes_store = MassaDbLsmtree::new(
            LSMTREE_NODES_CF,
            db.clone(),
            current_batch.clone(),
            current_hashmap.clone(),
        );
        let values_store = MassaDbLsmtree::new(
            LSMTREE_VALUES_CF,
            db.clone(),
            current_batch.clone(),
            current_hashmap.clone(),
        );

        let handle_metadata = db.cf_handle(METADATA_CF).expect(CF_ERROR);
        let lsmtree = match db
            .get_cf(handle_metadata, STATE_HASH_KEY)
            .expect(CRUD_ERROR)
        {
            Some(hash_bytes) => SparseMerkleTree::import(nodes_store, values_store, hash_bytes),
            _ => SparseMerkleTree::new_with_stores(nodes_store, values_store),
        };

        let massa_db = MassaDB {
            db,
            config,
            change_history: BTreeMap::new(),
            change_history_versioning: BTreeMap::new(),
            change_id_serializer: SlotSerializer::new(),
            change_id_deserializer,
            lsmtree,
            current_batch,
            current_hashmap,
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
