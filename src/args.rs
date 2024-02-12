use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(name = "massa-ledger-editor")]
#[command(about = "Ledger editor", long_about = None)]
pub struct Cli {
    #[arg(short = 'p', long = "path", help = "Path of an existing db")]
    pub(crate) path: PathBuf,
    #[arg(
        short = 'r',
        long = "initial_rolls_path",
        help = "Path of initial_rolls.json file (Massa node config file)"
    )]
    pub(crate) initial_rolls_path: PathBuf,
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Clone, PartialEq, Subcommand)]
pub(crate) enum Commands {
    #[command(about = "Convert ledger (from testnet 22 to testnet 23)")]
    ConvertLedger(ConvertLedgerArgs),
    #[command(about = "Edit ledger (Dummy impl for now)")]
    EditLedger,
    #[command(about = "Scan ledger (print trail hash + final state fingerprint")]
    ScanLedger,
    #[command(about = "Fill ledger with random values")]
    FillLedger(FillLedgerArgs),
    #[command(
        about = "Update MIP store (after a network shutdown) ensuring MIP info are coherent"
    )]
    UpdateMipStore(UpdateMipStoreArgs),
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct ConvertLedgerArgs {
    #[arg(
        short = 'o',
        long = "output_path",
        help = "Path where to write converted db"
    )]
    pub(crate) output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct FillLedgerArgs {
    #[arg(
        short = 's',
        long = "target_ledger_size",
        help = "Ledger size expected (ex: 10Mb, 1.5Gb)"
    )]
    pub(crate) target_ledger_size: String,
    #[arg(default_value = "205", help = "")]
    pub(crate) datastore_key_size: usize,
    #[arg(default_value = "9999989", help = "")]
    pub(crate) datastore_value_size: usize,
    #[arg(default_value = "9999989", help = "")]
    pub(crate) bytecode_size: usize,
}

#[derive(Debug, Clone, PartialEq, Args)]
pub struct UpdateMipStoreArgs {
    #[arg(long = "shutdown_start", help = "")]
    pub(crate) shutdown_start: u64,
    #[arg(long = "shutdown_end", help = "")]
    pub(crate) shutdown_end: u64,
    #[arg(long = "genesis_timestamp", help = "")]
    pub(crate) genesis_timestamp: u64,
}
