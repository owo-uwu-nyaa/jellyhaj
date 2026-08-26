use std::{
    fs::File,
    io::{Write, stdout},
    path::PathBuf,
    process::abort,
    sync::Mutex,
};

use clap::{Parser, Subcommand};
use color_eyre::{
    Section, SectionExt,
    eyre::{Context, OptionExt, Result, eyre},
};
use crossterm::{ExecutableCommand, style::Stylize, terminal::SetTitle};
use jellyhaj::run_app;
#[cfg(unix)]
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal};
use rayon::ThreadPoolBuilder;
use tracing::{error, error_span, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn log_stdout() -> Result<()> {
    let format = tracing_subscriber::fmt::format();
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .context("parsing log config from RUST_LOG")?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi_sanitization(false)
        .event_format(format)
        .with_filter(filter);
    let error_layer = ErrorLayer::default();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(error_layer)
        .try_init()
        .context("initializing tracing subscriber")?;
    Ok(())
}

fn log_file() -> Result<()> {
    let mut logfile = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .ok_or_eyre("unable to determine runtime or cache dir")?;
    logfile.push("jellyhaj.log");
    let format = tracing_subscriber::fmt::format();
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .context("parsing log config from RUST_LOG")?;
    #[cfg(feature = "journald")]
    let journal_filter = filter.clone();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi_sanitization(false)
        .with_writer(Mutex::new(
            File::create(&logfile).context("opening logfile")?,
        ))
        .event_format(format)
        .with_filter(filter);
    let error_layer = ErrorLayer::default();
    let registry = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(error_layer)
        .with(tui_logger::TuiTracingSubscriberLayer);
    #[cfg(feature = "console-subscriber")]
    let registry = registry.with(console_subscriber::spawn());
    #[cfg(feature = "journald")]
    let registry = registry.with(
        tracing_journald::layer()?
            .with_syslog_identifier("jellyhaj".to_string())
            .with_filter(journal_filter),
    );
    registry.init();
    println!("logging to {}", logfile.display());
    Ok(())
}

#[cfg(feature = "attach")]
fn attach() -> Result<()> {
    unsafe {
        libc::prctl(libc::PR_SET_PTRACER, libc::PR_SET_PTRACER_ANY);
    }
    let mut out = String::new();
    println!("attach to {}", std::process::id());
    println!("press enter to continue");
    std::io::stdin().read_line(&mut out)?;
    Ok(())
}

#[cfg(unix)]
extern "C" fn signal_handler(_: std::ffi::c_int) {
    std::thread::spawn(|| {
        jellyhaj_core::term::disable_unconditional();
        abort()
    });
}

#[cfg(unix)]
fn register_signal_handler() -> Result<()> {
    unsafe {
        nix::sys::signal::sigaction(
            Signal::SIGUSR1,
            &SigAction::new(
                SigHandler::Handler(signal_handler),
                SaFlags::SA_RESETHAND | SaFlags::SA_NODEFER,
                SigSet::empty(),
            ),
        )
    }?;
    Ok(())
}

fn main() -> Result<()> {
    unsafe { std::env::set_var("LC_NUMERIC", "C") };
    #[cfg(unix)]
    #[cfg(feature = "attach")]
    attach()?;
    let args = Args::parse();
    if args.features {
        println!("enabled features: {}", env!("JELLYFIN_TUI_FEATURES"));
        return Ok(());
    }
    match args.action {
        Some(Action::Print { what }) => {
            color_eyre::install().expect("installing color eyre format handler");
            match what {
                PrintAction::ConfigDir => println!(
                    "{}",
                    dirs::config_dir()
                        .ok_or_eyre("Couldn't determine user config dir")?
                        .display()
                ),
                PrintAction::Keybinds => {
                    stdout().write_all(include_bytes!("../config/keybinds.toml"))?;
                }
                PrintAction::Config => {
                    stdout().write_all(include_bytes!("../config/config.toml"))?;
                }
            }
            Ok(())
        }
        Some(Action::CheckKeybinds { file }) => {
            color_eyre::install().expect("installing color eyre format handler");
            log_stdout()?;
            config::check_keybinds_file(file)
        }
        Some(Action::CheckEffects { file }) => {
            color_eyre::install().expect("installing color eyre format handler");
            log_stdout()?;
            config::effects::EffectStore::parse(
                &std::fs::read_to_string(file).context("reading file")?,
            )?;
            Ok(())
        }
        Some(Action::CheckConfig { file }) => {
            color_eyre::install().expect("installing color eyre format handler");
            log_stdout()?;
            config::check_config_file(file)?;
            Ok(())
        }
        None => {
            log_file()?;
            tui_logger::init_logger(tui_logger::LevelFilter::Debug)
                .context("setting up tui logger")?;
            tui_logger::set_default_level(tui_logger::LevelFilter::Info);
            tui_logger::set_env_filter_from_env(None);
            let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::new().into_hooks();
            eyre_hook.install().expect("installing eyre hook");
            register_signal_handler()?;
            std::io::stdout().execute(SetTitle("jellyhaj"))?;
            let (panic_send, panic_recv) = tokio::sync::mpsc::unbounded_channel();
            std::panic::set_hook(Box::new(move |panic| {
                let _ = panic_send.send({
                    let report = eyre!("Application paniced");
                    if let Some(payload) = panic.payload_as_str() {
                        report.section(payload.to_string().header("Panic message"))
                    } else {
                        report.section("No panic message".red().bold().header("Panic message"))
                    }
                });
                let report = panic_hook.panic_report(panic);
                error!("{}", report);
            }));
            ThreadPoolBuilder::new()
                .thread_name(|n| format!("tui-worker-{n}"))
                .build_global()
                .context("building global thread pool")?;
            jellyhaj_core::term::run_with(|term| {
                spawn::run_with_spawner(
                    move |spawner| run_app(term, spawner, args.config, args.use_builtin_config),
                    error_span!("jellyhaj"),
                    "jellyhaj_main",
                    panic_recv,
                )
            })
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    action: Option<Action>,
    /// alternative config file
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[arg(short = 'b', long)]
    use_builtin_config: bool,
    #[arg(short, long)]
    features: bool,
}

#[derive(Debug, Subcommand)]
enum Action {
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
enum PrintAction {
    ConfigDir,
    Keybinds,
    Config,
}
