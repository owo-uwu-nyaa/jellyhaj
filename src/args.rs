#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub action: Option<Action>,
    /// alternative config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(short = 'b', long)]
    pub use_builtin_config: bool,
    #[arg(short, long)]
    pub features: bool,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    CheckKeybinds {
        /// keybinds config to check
        file: PathBuf,
    },
    CheckEffects {
        /// effects file to check
        file: PathBuf,
    },
    CheckConfig {
        /// effects file to check
        file: PathBuf,
    },

    Print {
        /// what should be printed
        #[command(subcommand)]
        what: PrintAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum PrintAction {
    ConfigDir,
    Keybinds,
    Config,
}
