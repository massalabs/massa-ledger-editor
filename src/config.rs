use massa_async_pool::AsyncPoolConfig;
use massa_db_exports::MassaDBConfig;
use massa_executed_ops::{ExecutedDenunciationsConfig, ExecutedOpsConfig};
use massa_final_state::FinalStateConfig;
use massa_ledger_exports::LedgerConfig;
use massa_models::config::{
    DENUNCIATION_EXPIRE_PERIODS, ENDORSEMENT_COUNT, GENESIS_TIMESTAMP, INITIAL_DRAW_SEED,
    MAX_ASYNC_POOL_LENGTH, MAX_BOOTSTRAP_VERSIONING_ELEMENTS_SIZE,
    MAX_BOOTSTRAP_FINAL_STATE_PARTS_SIZE, MAX_FUNCTION_NAME_LENGTH, MAX_PARAMETERS_SIZE,
    MAX_DATASTORE_KEY_LENGTH, MAX_DATASTORE_VALUE_LENGTH, MAX_DEFERRED_CREDITS_LENGTH,
    MAX_DENUNCIATIONS_PER_BLOCK_HEADER, MAX_DENUNCIATION_CHANGES_LENGTH,
    MAX_PRODUCTION_STATS_LENGTH, MAX_ROLLS_COUNT_LENGTH, MIP_STORE_STATS_BLOCK_CONSIDERED,
    PERIODS_PER_CYCLE, POS_SAVED_CYCLES, T0, THREAD_COUNT,
};
use massa_pos_exports::PoSConfig;
use massa_versioning::versioning::MipStatsConfig;
use num::rational::Ratio;
use std::path::PathBuf;

pub fn get_db_config(path: PathBuf) -> MassaDBConfig {
    MassaDBConfig {
        path,
        max_history_length: 100,
        max_versioning_elements_size: MAX_BOOTSTRAP_VERSIONING_ELEMENTS_SIZE as usize,
        max_final_state_elements_size: MAX_BOOTSTRAP_FINAL_STATE_PARTS_SIZE as usize,
        thread_count: THREAD_COUNT,
    }
}

pub fn get_ledger_config(path: PathBuf) -> LedgerConfig {
    LedgerConfig {
        thread_count: THREAD_COUNT,
        disk_ledger_path: path,
        initial_ledger_path: PathBuf::new(),
        max_key_length: MAX_DATASTORE_KEY_LENGTH,
        max_datastore_value_length: MAX_DATASTORE_VALUE_LENGTH,
    }
}

fn get_async_pool_config() -> AsyncPoolConfig {
    AsyncPoolConfig {
        max_length: MAX_ASYNC_POOL_LENGTH,
        thread_count: THREAD_COUNT,
        max_function_length: MAX_FUNCTION_NAME_LENGTH,
        max_function_params_length: MAX_PARAMETERS_SIZE as u64,
        max_key_length: MAX_DATASTORE_KEY_LENGTH as u32,
    }
}

fn get_pos_config() -> PoSConfig {
    PoSConfig {
        initial_deferred_credits_path: Some(PathBuf::new()),
        periods_per_cycle: PERIODS_PER_CYCLE,
        thread_count: THREAD_COUNT,
        cycle_history_length: POS_SAVED_CYCLES,
        max_rolls_length: MAX_ROLLS_COUNT_LENGTH,
        max_production_stats_length: MAX_PRODUCTION_STATS_LENGTH,
        max_credit_length: MAX_DEFERRED_CREDITS_LENGTH,
    }
}

fn get_executed_ops_config() -> ExecutedOpsConfig {
    ExecutedOpsConfig {
        keep_executed_history_extra_periods: 1,
        thread_count: THREAD_COUNT,
    }
}

fn get_executed_denunciations_config() -> ExecutedDenunciationsConfig {
    ExecutedDenunciationsConfig {
        keep_executed_history_extra_periods: 1,
        denunciation_expire_periods: DENUNCIATION_EXPIRE_PERIODS,
        thread_count: THREAD_COUNT,
        endorsement_count: ENDORSEMENT_COUNT,
    }
}

pub fn get_mip_stats_config() -> MipStatsConfig {
    MipStatsConfig {
        block_count_considered: MIP_STORE_STATS_BLOCK_CONSIDERED,
        warn_announced_version_ratio: Ratio::new(3, 10),
    }
}

pub fn get_final_state_config(
    path: PathBuf,
    initial_rolls_path: Option<PathBuf>,
) -> FinalStateConfig {
    let ledger_config = get_ledger_config(path.clone());
    let async_pool_config = get_async_pool_config();
    let pos_config = get_pos_config();
    let executed_ops_config = get_executed_ops_config();
    let executed_denunciations_config = get_executed_denunciations_config();

    let initial_rolls_path = match initial_rolls_path {
        Some(p) => p,
        None => path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("base_config")
            .join("initial_rolls.json"),
    };
    println!("initial_rolls_path: {:?}", initial_rolls_path);
    FinalStateConfig {
        ledger_config,
        async_pool_config,
        pos_config,
        executed_ops_config,
        executed_denunciations_config,
        final_history_length: 100,
        thread_count: THREAD_COUNT,
        periods_per_cycle: PERIODS_PER_CYCLE,
        initial_seed_string: INITIAL_DRAW_SEED.into(),
        initial_rolls_path,
        endorsement_count: ENDORSEMENT_COUNT,
        max_executed_denunciations_length: MAX_DENUNCIATION_CHANGES_LENGTH,
        max_denunciations_per_block_header: MAX_DENUNCIATIONS_PER_BLOCK_HEADER,
        t0: T0,
        genesis_timestamp: *GENESIS_TIMESTAMP,
    }
}
