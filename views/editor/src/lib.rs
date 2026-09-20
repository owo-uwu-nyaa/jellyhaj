use std::{borrow::Cow, ops::ControlFlow, sync::Arc};

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
    ContextRef, GetFromContext,
    async_task::ErasedSubmitter,
    outer::{Named, OuterWidget},
    spawn::Spawner,
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
    if let Some(editor) = config.editor.as_ref() {
        let _editor = editor.clone();
        let task = async move { todo!() };
        make_new_erased(
            cx,
            ReadyWidget::new(Box::new(Navigation::PushWithoutTui(Box::pin(task)))),
        )
    } else {
        let widget = Editor::new(title, &text, res);
        let widget = KeybindWidget::new(widget, config.keybinds.editor.clone(), Mapper);
        let widget = OuterWidget::<Name, _>::new(widget);
        make_new_erased(cx, widget)
    }
}
