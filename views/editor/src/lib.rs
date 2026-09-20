use std::{
    borrow::Cow,
    env::var_os,
    ffi::OsString,
    mem::{self, ManuallyDrop},
    ops::{ControlFlow, Deref},
    path::PathBuf,
    sync::Arc,
};

use eyre::{WrapErr, bail, eyre};

use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::EditorCommand,
    state::Navigation,
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_editor_widget::{Editor, EditorAction};
use jellyhaj_helper_widget::ReadyWidget;
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, Result,
    async_task::ErasedSubmitter,
    outer::{Named, OuterWidget},
    spawn::{
        Spawner,
        tracing::{error, warn},
    },
};

struct Mapper;
impl CommandMapper<EditorCommand> for Mapper {
    type A = EditorAction;

    fn map(&self, command: EditorCommand) -> std::ops::ControlFlow<Navigation, Self::A> {
        let action = match command {
            EditorCommand::Finish => EditorAction::Finish,
            EditorCommand::NewLine => EditorAction::NewLine,
            EditorCommand::Delete => EditorAction::Delete,
            EditorCommand::Up => EditorAction::Up,
            EditorCommand::Down => EditorAction::Down,
            EditorCommand::Left => EditorAction::Left,
            EditorCommand::Right => EditorAction::Right,
            EditorCommand::Quit => EditorAction::Quit,
            EditorCommand::Global(global_command) => {
                return ControlFlow::Break(global_command.into());
            }
        };
        ControlFlow::Continue(action)
    }
}

struct Name;
impl Named for Name {
    const NAME: &str = "editor";
}

#[allow(clippy::needless_pass_by_value)]
pub fn make_editor(
    cx: impl ContextRef<Config> + ContextRef<Spawner> + 'static,
    title: Cow<'static, str>,
    text: String,
    res: Arc<dyn ErasedSubmitter<String>>,
) -> Erased {
    let config = Config::get_ref(&cx);
    if config.editor.is_empty() {
        let widget = Editor::new(title, &text, res);
        let widget = KeybindWidget::new(widget, config.keybinds.editor.clone(), Mapper);
        let widget = OuterWidget::<Name, _>::new(widget);
        make_new_erased(cx, widget)
    } else {
        let editor = config.editor.clone();
        let task = async move {
            let dir = make_dir().await?;
            let mut file = dir.clone();
            file.push(title.into_owned());
            tokio::fs::write(&file, text)
                .await
                .context("writing initial file content")?;
            let status = tokio::process::Command::new(&editor[0])
                .args(&editor[1..])
                .arg(file.as_os_str())
                .spawn()
                .context("spawning editor as hild process")?
                .wait()
                .await
                .context("waiting for editor to finish")?;
            if !status.success() {
                bail!("editor exited with exit status != 0");
            }
            let content = tokio::fs::read_to_string(&file)
                .await
                .context("reading editor result")?;
            res.spawn_value_infallible(content);
            dir.delete().await
        };
        make_new_erased(
            cx,
            ReadyWidget::new(Box::new(Navigation::PushWithoutTui(Box::pin(task)))),
        )
    }
}

#[allow(clippy::unnecessary_wraps)]
#[cfg(unix)]
fn default_tmp() -> Option<OsString> {
    Some("/tmp".to_owned().into())
}

#[cfg(not(unix))]
fn default_tmp() -> Option<OsString> {
    None
}

struct TmpDir(PathBuf);

impl TmpDir {
    async fn delete(self) -> Result<()> {
        let mut this = ManuallyDrop::new(self);
        let this = mem::take(&mut this.0);
        tokio::fs::remove_dir_all(&this)
            .await
            .wrap_err("removing temporary dir")
    }
}

impl Deref for TmpDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let path = std::mem::take(&mut self.0);
        tokio::task::spawn_blocking(move || {
            if let Err(e) = std::fs::remove_dir_all(&path) {
                error!("error deleting temporary dir: {e:?}");
            }
        });
    }
}

async fn make_dir() -> Result<TmpDir> {
    if let Some(dir) = var_os("TMPDIR")
        .or_else(|| var_os("TMP"))
        .or_else(default_tmp)
    {
        let dir: PathBuf = dir.into();
        let mut try_n = 0;
        let dir = loop {
            try_n += 1;
            let id = getrandom::u64().wrap_err("generating random dir name")?;
            let name: OsString = format!("jellyhaj-{id}").into();
            let mut dir = dir.clone();
            dir.push(name);
            match tokio::fs::create_dir(&dir).await {
                Ok(()) => break dir,
                Err(e) if try_n > 3 => {
                    return Err(e).wrap_err("creating temp dir failed the 3rd time!");
                }
                Err(e) => {
                    warn!("creating temp dir failed: {e:?}");
                }
            }
        };
        Ok(TmpDir(dir))
    } else {
        Err(eyre!("Could not find system tmpdir"))
    }
}
