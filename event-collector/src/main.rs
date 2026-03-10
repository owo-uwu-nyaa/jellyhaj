use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
    pin::{Pin, pin},
};

use clap::{Parser, crate_name, crate_version};
use color_eyre::{Result, eyre::Context};
use config::LoginInfo;
use futures_util::{Stream, StreamExt};
use jellyfin::{ClientInfo, socket::JellyfinMessage};
use tokio::select;

fn read(path: &Path) -> Option<Vec<JellyfinMessage>> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?
}

async fn next(
    mut socket: Pin<&mut impl Stream<Item = JellyfinMessage>>,
    cancel: Pin<&mut impl Future<Output = io::Result<()>>>,
) -> Result<Option<JellyfinMessage>> {
    select! {
        m = cancel => {
            m?;
            Ok(None)
        }
        m = socket.next() => {
            Ok(m)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    color_eyre::install()?;
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .context("parsing log config from RUST_LOG")?;
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let config = config::init_config(args.config, args.use_builtin_config)?;
    let login: LoginInfo = toml::from_str(&std::fs::read_to_string(config.login_file)?)?;
    let device_name: Cow<'static, str> = whoami::hostname()
        .ok()
        .map(|v| v.into())
        .unwrap_or_else(|| "unknown".into());

    let password = login.get_password().await?;
    let client = jellyfin::JellyfinClient::new_auth_name(
        login.server_url,
        ClientInfo {
            name: crate_name!().into(),
            version: crate_version!().into(),
        },
        device_name,
        login.username,
        password,
        1,
    )
    .await?;
    let socket = pin!(client.get_socket()?);
    let mut info = read(&args.output).unwrap_or_default();
    let res = collect(socket, &mut info, &args.output).await;
    fs::write(args.output, serde_json::to_vec(&info)?)?;
    res
}

async fn collect(
    mut socket: Pin<&mut impl Stream<Item = JellyfinMessage>>,
    info: &mut Vec<JellyfinMessage>,
    output: &Path,
) -> Result<()> {
    let mut cancel = pin!(tokio::signal::ctrl_c());
    while let Some(m) = next(socket.as_mut(), cancel.as_mut()).await? {
        info.push(m);
        fs::write(output, serde_json::to_vec(info)?)?;
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// alternative config file
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[arg(short = 'b', long)]
    use_builtin_config: bool,
    output: PathBuf,
}
