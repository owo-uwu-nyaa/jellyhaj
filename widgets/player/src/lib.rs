use color_eyre::eyre::{Context, bail};
use futures_util::stream::unfold;
use jellyfin::items::ItemType;
use jellyhaj_core::{state::Navigation, widgets::KeybindAction};
use jellyhaj_widgets_core::{JellyhajWidget, JellyhajWidgetBase, Result, WidgetContext, Wrapper};
use player_core::{
    Command, Events, PlayerHandle, PlayerState,
    state::{EventReceiver, SharedPlayerState},
};
use ratatui::{
    buffer::CellWidth,
    crossterm::event::MouseEventKind,
    layout::Constraint,
    prelude::Rect,
    widgets::{Block, Gauge, Padding, Paragraph, Widget},
};
use tracing::{info_span, instrument};
use valuable::Valuable;

#[derive(Valuable)]
pub struct PlayerWidget {
    #[valuable(skip)]
    handle: PlayerHandle,
    state: Option<SharedPlayerState>,
    labels: Vec<String>,
}

impl Drop for PlayerWidget {
    fn drop(&mut self) {
        self.handle.send(Command::Stop);
    }
}

impl PlayerWidget {
    #[must_use]
    pub fn new(handle: PlayerHandle) -> Self {
        Self {
            handle,
            state: None,
            labels: vec!["Waiting for player state".to_owned()],
        }
    }
}

#[derive(Debug)]
pub enum PlayerAction {
    Quit,
    Forward,
    Backward,
    Next,
    Prev,
    TogglePause,
    Update,
    Events(EventReceiver),
}

#[derive(Debug)]
pub struct PlayerQuit;

impl JellyhajWidgetBase for PlayerWidget {
    type Action = PlayerAction;

    type ActionResult = Navigation;

    const NAME: &str = "player";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        Some(27)
    }
    fn min_height(&self) -> Option<u16> {
        Some(15)
    }
}

fn make_lables(state: &PlayerState) -> Result<Vec<String>> {
    let res = if let Some(index) = state.current {
        let media_item = &state.playlist[index].item;
        match &media_item.item_type {
            ItemType::Movie | ItemType::Audio => {
                vec![media_item.name.clone()]
            }
            ItemType::Music {
                album_id: _,
                album: album_name,
            } => {
                vec![media_item.name.clone(), album_name.clone()]
            }
            ItemType::Episode {
                season_id: _,
                season_name: None,
                series_id: _,
                series_name,
            } => {
                let mut series_str = series_name.clone();
                if media_item.episode_index.is_some() || media_item.season_index.is_some() {
                    series_str.push(' ');
                    if let Some(season) = media_item.season_index {
                        series_str.push('S');
                        series_str.push_str(&season.to_string());
                    }
                    if let Some(episode) = media_item.episode_index {
                        series_str.push('E');
                        series_str.push_str(&episode.to_string());
                    }
                }
                vec![series_str, media_item.name.clone()]
            }
            ItemType::Episode {
                season_id: _,
                season_name: Some(season_name),
                series_id: _,
                series_name,
            } => {
                let mut series_str = series_name.clone();
                if media_item.episode_index.is_some() || media_item.season_index.is_some() {
                    series_str.push(' ');
                    if let Some(season) = media_item.season_index {
                        series_str.push('S');
                        series_str.push_str(&season.to_string());
                    }
                    if let Some(episode) = media_item.episode_index {
                        series_str.push('E');
                        series_str.push_str(&episode.to_string());
                    }
                }
                vec![series_str, season_name.clone(), media_item.name.clone()]
            }
            _ => {
                bail!("Unexpected media item type: {media_item:#?}");
            }
        }
    } else {
        vec!["Nothing is currently playing".to_owned()]
    };
    Ok(res)
}

impl<R: 'static> JellyhajWidget<R> for PlayerWidget {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        let res = self.handle.get_state();
        cx.submitter.spawn_task(
            async move {
                Ok(PlayerAction::Events(
                    res.await.context("receiving player event receiver")?,
                ))
            },
            info_span!("get_event_receiver"),
            "get_event_receiver",
        );
    }

    #[instrument(name = "apply_action_player", skip_all)]
    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        match action {
            PlayerAction::Quit => Ok(Some(Navigation::PopContext)),
            PlayerAction::TogglePause => {
                self.handle.send(Command::TogglePause);
                Ok(None)
            }
            PlayerAction::Forward => {
                self.handle.send(Command::SeekRelative(15.0));
                Ok(None)
            }
            PlayerAction::Backward => {
                self.handle.send(Command::SeekRelative(-15.0));
                Ok(None)
            }
            PlayerAction::Next => {
                self.handle.send(Command::Next);
                Ok(None)
            }
            PlayerAction::Prev => {
                self.handle.send(Command::Previous);
                Ok(None)
            }
            PlayerAction::Update => {
                if let Some(state) = self.state.as_ref() {
                    self.labels = make_lables(&state.lock())?;
                }
                Ok(None)
            }
            PlayerAction::Events(event_receiver) => {
                let receiver = event_receiver.with_shared_state();
                self.state = Some(SharedPlayerState::clone(&receiver));
                cx.submitter.spawn_stream(
                    unfold(receiver, |mut receiver| async {
                        loop {
                            let action = receiver
                                .receive_inspect(async |event, state| match event {
                                    Events::Duration(_)
                                    | Events::ReplacePlaylist { .. }
                                    | Events::AddPlaylistItem { .. }
                                    | Events::RemovePlaylistItem { .. }
                                    | Events::Current(_) => Some(PlayerAction::Update),
                                    Events::Position(p) => {
                                        let floored = p.floor();
                                        let lock = state.lock();
                                        let prev = lock.position.floor();
                                        if lock.pause || floored > prev {
                                            Some(PlayerAction::Update)
                                        } else {
                                            None
                                        }
                                    }
                                    Events::Paused(_)
                                    | Events::Stopped(false)
                                    | Events::Seek(_)
                                    | Events::Speed(_)
                                    | Events::Fullscreen(_)
                                    | Events::Volume(_) => None,
                                    Events::Stopped(true) => Some(PlayerAction::Quit),
                                })
                                .await
                                .context("receiving player events");
                            match action {
                                Ok(Some(action)) => break Some((Ok(action), receiver)),
                                Ok(None) => {}
                                Err(e) => break Some((Err(e), receiver)),
                            }
                        }
                    }),
                    info_span!("recv_player_state"),
                    "recv_player_state",
                );
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        _: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        pos: ratatui::prelude::Position,
        size: ratatui::prelude::Size,
        kind: MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if kind.is_down()
            && let Some(state) = self.state.as_ref()
        {
            if pos.y == size.height - 3 && pos.x > 1 && pos.x < (size.width - 2) {
                let pos = f64::from(pos.x - 2);
                let len = f64::from(size.width - 4);
                let pos = pos / len * state.lock().duration;
                self.handle.send(Command::Seek(pos));
            } else {
                self.handle.send(Command::TogglePause);
            }
        }
        Ok(None)
    }

    #[instrument(name = "render_player", skip_all)]
    fn render_fallible_inner(
        &mut self,
        mut area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        let block = Block::bordered()
            .title("Now playing")
            .padding(Padding::uniform(1));
        let main = block.inner(area);
        assert!(self.labels.len() <= 3);
        let height = u16::try_from(self.labels.len()).expect("should not overflow") * 2 + 1;
        let inner = main.centered_vertically(Constraint::Length(height));
        for (label, y) in self.labels.iter().zip(0u16..) {
            let area = Rect {
                x: inner.x,
                y: inner.y.strict_add(1).strict_add(y.strict_mul(2)),
                width: inner.width,
                height: 1,
            };
            Paragraph::new(label.as_str()).centered().render(area, buf);
        }
        if let Some(state) = self.state.as_ref() {
            let state = state.lock();
            let mut dur_line = main;
            dur_line.y += main.height - 2;
            dur_line.height = 1;
            let position = state.position.floor();
            let duration = state.duration.ceil();
            secs_to_str(position).render(dur_line, buf);
            Paragraph::new(secs_to_str((duration - position).round()))
                .centered()
                .render(dur_line, buf);
            let dur = secs_to_str(duration);
            let dur_len = dur.cell_width();
            dur_line.x += dur_line.width - (dur_len);
            dur_line.width = dur_len;
            dur.render(dur_line, buf);

            let mut timeline_pos = main;
            timeline_pos.y += main.height - 1;
            timeline_pos.height = 1;
            Gauge::default()
                .use_unicode(true)
                .ratio((position / duration).min(1.0))
                .render(timeline_pos, buf);
            block.render(area, buf);
            let play_state = if state.stopped {
                "Stopped"
            } else if state.pause {
                "Paused"
            } else {
                "Playing"
            };
            let play_state_len = play_state.cell_width();
            area.x += area.width - 1 - play_state_len;
            area.height = 1;
            area.width = play_state_len;
            play_state.render(area, buf);
        } else {
            block.render(area, buf);
        }
        Ok(())
    }
}
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn secs_to_str(secs: f64) -> String {
    let mut total_secs = secs as u32;
    let secs = total_secs % 60;
    total_secs /= 60;
    let mins = total_secs % 60;
    total_secs /= 60;
    let hours = total_secs;
    format!("{hours}:{mins:02}:{secs:02}")
}

#[derive(Debug)]
pub enum ExitAction {
    Quit,
}
#[derive(Valuable)]
pub struct ExitWidget;

impl JellyhajWidgetBase for ExitWidget {
    type Action = KeybindAction<ExitAction>;

    type ActionResult = Navigation;

    const NAME: &str = "quit";

    fn visit_children(&self, _: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        None
    }

    fn min_height(&self) -> Option<u16> {
        None
    }
}

impl<R: 'static> JellyhajWidget<R> for ExitWidget {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        cx.submitter
            .spawn_value_infallible(KeybindAction::Inner(ExitAction::Quit));
    }

    fn apply_action(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        if matches!(action, KeybindAction::Inner(ExitAction::Quit)) {
            Ok(Some(Navigation::Exit))
        } else {
            Ok(None)
        }
    }

    fn click(
        &mut self,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _position: ratatui::prelude::Position,
        _size: ratatui::prelude::Size,
        _kind: MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        Paragraph::new("stopping").centered().render(area, buf);
        Ok(())
    }
}
