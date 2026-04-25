use std::{path::PathBuf, str::FromStr};

use color_eyre::eyre::{Context, OptionExt, Result, eyre};
use libmpv::MpvProfile;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

pub use cache::cache;
pub use keybinds::check_keybinds_file;

use crate::{effects::EffectStore, keybind_defs::Keybinds};

mod cache;
pub mod effects;
pub mod keybind_defs;
mod keybinds;

#[derive(Debug)]
pub struct Config {
    pub hwdec: String,
    pub keybinds: Keybinds,
    pub login_file: PathBuf,
    pub mpv_log_level: String,
    pub mpv_profile: MpvProfile,
    pub help_prefixes: Vec<String>,
    pub mpv_config_file: Option<PathBuf>,
    pub entry_image_width: u16,
    pub concurrent_jellyfin_connections: u8,
    pub fetch_timeout: u16,
    pub effects: EffectStore,
    pub store_access_token: bool,
}

#[derive(Debug, Deserialize)]
struct ParseConfig {
    pub login_file: Option<PathBuf>,
    pub keybinds_file: Option<PathBuf>,
    pub effects_file: Option<PathBuf>,
    pub hwdec: String,
    pub mpv_profile: Option<String>,
    pub mpv_log_level: String,
    pub mpv_config_file: Option<PathBuf>,
    pub entry_image_width: Option<u16>,
    pub concurrent_jellyfin_connections: Option<u8>,
    pub fetch_timeout: Option<u16>,
    #[serde(default)]
    pub store_access_token: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginInfo {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub password_cmd: Option<Vec<String>>,
}

impl LoginInfo {
    pub async fn get_password(&self) -> Result<String> {
        if let Some(cmd) = self.password_cmd.as_ref() {
            let mut command = if let Some(cmd) = cmd.first() {
                tokio::process::Command::new(cmd)
            } else {
                return Err(eyre!("Password cmd is empty"));
            };
            for arg in cmd[1..].iter() {
                command.arg(arg);
            }
            let output = command
                .kill_on_drop(true)
                .output()
                .await
                .context("Executing password cmd failed")?;
            if output.status.success() {
                Ok(String::from_utf8(output.stdout)
                    .context("password cmd output is not utf-8")?
                    .trim()
                    .to_string())
            } else {
                Err(eyre!(
                    "command failed with:\n{}",
                    String::from_utf8(output.stderr)
                        .context("password cmd error output is not utf-8")?
                ))
            }
        } else {
            Ok(self.password.clone())
        }
    }
}

#[instrument]
pub fn init_config(config_file: Option<PathBuf>, use_builtin: bool) -> Result<Config> {
    let (config_dir, config_file) = if let Some(config_file) = config_file {
        (
            config_file
                .parent()
                .ok_or_eyre("config file has no parent directory")?
                .to_path_buf(),
            config_file,
        )
    } else {
        let mut config_dir = dirs::config_dir().ok_or_eyre("Couldn't determine user config dir")?;
        config_dir.push("jellyhaj");
        let mut config_file = config_dir.clone();
        config_file.push("config.toml");
        (config_dir, config_file)
    };
    if !use_builtin {
        info!("loading config from {}.", config_file.display());
    } else {
        info!("loading built in config.")
    }

    let config: ParseConfig = if !use_builtin && config_file.exists() {
        toml::from_str(&std::fs::read_to_string(config_file).context("reading config file")?)
    } else {
        toml::from_str(include_str!("../config.toml"))
    }
    .context("parsing config")?;

    let default_keybinds = keybinds::from_str(include_str!("../keybinds.toml"), false)
        .context("parsing default keybinds")?;
    let (keybinds, help_prefixes) = if let Some(keybinds_file) = config.keybinds_file {
        let keybinds = if keybinds_file.is_absolute() {
            keybinds_file
        } else {
            let mut file = config_dir.clone();
            file.push(keybinds_file);
            file
        };
        keybinds::from_file(keybinds, false, default_keybinds.0).context("parsing keybindings")?
    } else if !use_builtin
        && let mut keybinds_file = config_dir.clone()
        && let _ = keybinds_file.push("keybinds.toml")
        && keybinds_file.exists()
    {
        keybinds::from_file(keybinds_file, false, default_keybinds.0)
            .context("parsing keybindings")?
    } else {
        default_keybinds
    };

    let effects = if let Some(effects_file) = config.effects_file {
        let file = if effects_file.is_absolute() {
            effects_file
        } else {
            let mut file = config_dir.clone();
            file.push(effects_file);
            file
        };
        let content = std::fs::read_to_string(file).context("reading effects file")?;
        EffectStore::parse(&content)
    } else if !use_builtin
        && let mut effects_file = config_dir.clone()
        && let _ = effects_file.push("effects.toml")
        && effects_file.exists()
    {
        let content = std::fs::read_to_string(effects_file).context("reading effects file")?;
        EffectStore::parse(&content)
    } else {
        EffectStore::parse(include_str!("../effects.toml"))
    }
    .context("parsing effects")?;

    let mpv_profile = config
        .mpv_profile
        .as_deref()
        .map(MpvProfile::from_str)
        .unwrap_or(Ok(MpvProfile::default()))
        .context("parsing mpv_profile")?;

    let login_file = if let Some(login_file) = config.login_file {
        if login_file.is_absolute() {
            login_file
        } else {
            let mut file = config_dir;
            file.push(&login_file);
            file
        }
    } else {
        let mut login_file = config_dir;
        login_file.push("login.toml");
        login_file
    };

    Ok(Config {
        login_file,
        hwdec: config.hwdec,
        keybinds,
        mpv_log_level: config.mpv_log_level,
        mpv_profile,
        help_prefixes,
        mpv_config_file: config.mpv_config_file,
        entry_image_width: config.entry_image_width.unwrap_or(32),
        concurrent_jellyfin_connections: config.concurrent_jellyfin_connections.unwrap_or(2),
        fetch_timeout: config.fetch_timeout.unwrap_or(15),
        effects,
        store_access_token: config.store_access_token,
    })
}

#[cfg(test)]
mod tests {
    use crate::init_config;
    use color_eyre::Result;
    #[test]
    fn check_default_config() -> Result<()> {
        init_config(None, true)?;
        Ok(())
    }
}
