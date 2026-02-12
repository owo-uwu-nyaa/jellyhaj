use futures_util::StreamExt;
use jellyhaj_loading_widget::{AdvanceLoadingScreen, Loading};
use jellyhaj_widgets_core::{
    JellyhajWidget, JellyhajWidgetExt,
    async_task::new_task_pair,
    spawn::{CancellationToken, run_with_spawner},
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{Event, EventStream, KeyCode, KeyEvent},
};
use tracing::info_span;

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn main() {
    let term = ratatui::init();
    let guard = Guard;
    run_widget(term);
    drop(guard);
}

#[tokio::main(flavor = "current_thread")]
async fn run_widget(mut term: DefaultTerminal) {
    run_with_spawner(
        |s| async move {
            let mut events = EventStream::new();
            let (task, _) = new_task_pair(s);
            let mut widget = Loading::new("press enter to advance, q to quit");
            loop {
                term.draw(|frame| {
                    widget
                        .render_fallible(frame.area(), frame.buffer_mut(), task.clone())
                        .expect("should not fail");
                })
                .expect("should not fail");
                loop {
                    match events
                        .next()
                        .await
                        .expect("events closed")
                        .expect("reading events failed")
                    {
                        Event::Resize(_, _) => continue,
                        Event::Key(KeyEvent {
                            code: KeyCode::Enter,
                            ..
                        }) => break,
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            ..
                        }) => return,
                        _ => {}
                    }
                }
                widget
                    .apply_action(task.clone(), AdvanceLoadingScreen)
                    .expect("applying action failed");
            }
        },
        CancellationToken::new(),
        info_span!("spawner"),
    )
    .await;
}
