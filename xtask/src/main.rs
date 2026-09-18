use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Generator, aot, generate_to};
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    match Args::parse().action {
        Action::PrintCompletions { shell, dir } => {
            std::fs::create_dir_all(&dir)?;
            for shell in shell {
                match shell {
                    Shell::Bash => generate(&dir, aot::Bash),
                    Shell::Fish => generate(&dir, aot::Fish),
                    Shell::PowerShell => generate(&dir, aot::PowerShell),
                    Shell::Zsh => generate(&dir, aot::Zsh),
                    Shell::Nushell => generate(&dir, clap_complete_nushell::Nushell),
                }?;
            }
            Ok(())
        }
    }
}

fn generate(dir: &Path, generator: impl Generator) -> Result<()> {
    generate_to(
        generator,
        &mut jellyhaj_cli::Args::command(),
        "jellyhaj",
        dir,
    )?;
    Ok(())
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
enum Action {
    PrintCompletions {
        dir: PathBuf,
        #[arg(value_enum, required(true))]
        shell: Vec<Shell>,
    },
}

#[derive(ValueEnum, Clone)]
enum Shell {
    Bash,
    Fish,
    #[allow(clippy::enum_variant_names)]
    PowerShell,
    Zsh,
    Nushell,
}

mod jellyhaj_cli {
    use clap::{Parser, Subcommand};
    use std::path::PathBuf;
    include!("../../src/args.rs");
}
