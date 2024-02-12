use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::BTreeMap;

use massa_ledger_exports::{LedgerChanges, LedgerEntry, SetUpdateOrDelete};
use massa_models::address::Address;
use massa_models::amount::Amount;
use massa_models::bytecode::Bytecode;
use massa_signature::KeyPair;

use crate::FillLedgerArgs;

#[inline]
fn generate_random_vector(size: usize, rng: &mut ThreadRng) -> Vec<u8> {
    (0..size).map(|_| rng.gen::<u8>()).collect::<Vec<u8>>()
}

pub fn create_ledger_entry(
    args: &FillLedgerArgs,
    changes: &mut LedgerChanges,
    rng: &mut ThreadRng,
) -> usize {
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
