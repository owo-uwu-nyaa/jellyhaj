use std::borrow::Cow;

use color_eyre::eyre::Context;
use futures_util::stream::unfold;
use jellyhaj_widgets_core::{JellyhajWidget, Wrapper, async_task::TaskSubmitter};
use player_core::{
    Command, Events, PlayerHandle,
    state::{EventReceiver, SharedPlayerState},
};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Block, Padding, Paragraph, Widget},
};
use tracing::{info_span, instrument};

pub struct PlayerWidget {
    handle: PlayerHandle,
    state: Option<SharedPlayerState>,
    send: bool,
}

impl PlayerWidget {
    pub fn new(handle: PlayerHandle) -> Self {
        Self {
            handle,
            state: None,
            send: false,
        }
    }
}

pub enum PlayerAction {
    Quit,
    TogglePause,
    Update,
    Events(EventReceiver),
}

pub struct PlayerQuit;

impl JellyhajWidget for PlayerWidget {
    type State = PlayerHandle;

    type Action = PlayerAction;

    type ActionResult = PlayerQuit;

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(5)
    }

    fn into_state(self) -> Self::State {
        self.handle
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {
        unimplemented!()
    }

    fn accept_text(&mut self, _: String) {
        unimplemented!()
    }

    #[instrument(name = "apply_action_player", skip_all)]
    fn apply_action(
        &mut self,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            PlayerAction::Quit => Ok(Some(PlayerQuit)),
            PlayerAction::TogglePause => {
                self.handle.send(Command::TogglePause);
                Ok(None)
            }
            PlayerAction::Update => Ok(None),
            PlayerAction::Events(event_receiver) => {
                let receiver = event_receiver.with_shared_state();
                self.state = Some(SharedPlayerState::clone(&receiver));
                task.spawn_stream(
                    unfold(receiver, |mut receiver| async {
                        loop {
                            let action = receiver
                                .receive_inspect(async |event, _| match event {
                                    Events::ReplacePlaylist { .. }
                                    | Events::AddPlaylistItem { .. }
                                    | Events::RemovePlaylistItem { .. }
                                    | Events::Current(_) => Some(PlayerAction::Update),
                                    Events::Paused(_)
                                    | Events::Stopped(false)
                                    | Events::Position(_)
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
                    info_span!("recv_state"),
                );
                Ok(None)
            }
        }
    }

    fn click(
        &mut self,
        _: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
        _: ratatui::prelude::Position,
        _: ratatui::prelude::Size,
        _: ratatui::crossterm::event::MouseEventKind,
        _: ratatui::crossterm::event::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        Ok(None)
    }

    #[instrument(name = "render_player", skip_all)]
    fn render_fallible_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        task: TaskSubmitter<Self::Action, impl Wrapper<Self::Action>>,
    ) -> jellyhaj_widgets_core::Result<()> {
        if !self.send {
            self.send = true;
            let res = self.handle.get_state();
            task.spawn_task(
                async move {
                    Ok(PlayerAction::Events(
                        res.await.context("receiving player event receiver")?,
                    ))
                },
                info_span!("get_event_receiver"),
            )
        }
        let block = Block::bordered()
            .title("Now playing")
            .padding(Padding::uniform(1));
        let main = block.inner(area);

        if let Some(state) = self.state.as_ref() {
            let state = state.lock();
            if let Some(index) = state.current {
                let media_item = &state.playlist[index].item;
                match &media_item.item_type {
                    jellyfin::items::ItemType::Movie => {
                        Paragraph::new(media_item.name.clone())
                            .centered()
                            .render(area, buf);
                    }
                    jellyfin::items::ItemType::Episode {
                        season_id: _,
                        season_name: None,
                        series_id: _,
                        series_name,
                    } => {
                        let [series, episode] =
                            Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)])
                                .vertical_margin(3)
                                .areas(main);
                        let mut series_str = Cow::from(series_name.as_str());
                        if media_item.episode_index.is_some() || media_item.season_index.is_some() {
                            series_str.to_mut().push(' ');
                            if let Some(season) = media_item.season_index {
                                series_str.to_mut().push('S');
                                series_str.to_mut().push_str(&season.to_string());
                            }
                            if let Some(episode) = media_item.episode_index {
                                series_str.to_mut().push('E');
                                series_str.to_mut().push_str(&episode.to_string());
                            }
                        }

                        Paragraph::new(series_str).centered().render(series, buf);
                        Paragraph::new(media_item.name.clone())
                            .centered()
                            .render(episode, buf);
                    }
                    jellyfin::items::ItemType::Episode {
                        season_id: _,
                        season_name: Some(season_name),
                        series_id: _,
                        series_name,
                    } => {
                        let [series, season, episode] = Layout::vertical([
                            Constraint::Fill(1),
                            Constraint::Fill(1),
                            Constraint::Fill(1),
                        ])
                        .vertical_margin(3)
                        .areas(main);
                        let mut series_str = Cow::from(series_name.as_str());
                        if media_item.episode_index.is_some() || media_item.season_index.is_some() {
                            series_str.to_mut().push(' ');
                            if let Some(season) = media_item.season_index {
                                series_str.to_mut().push('S');
                                series_str.to_mut().push_str(&season.to_string());
                            }
                            if let Some(episode) = media_item.episode_index {
                                series_str.to_mut().push('E');
                                series_str.to_mut().push_str(&episode.to_string());
                            }
                        }
                        Paragraph::new(series_str).centered().render(series, buf);
                        Paragraph::new(season_name.clone())
                            .centered()
                            .render(season, buf);
                        Paragraph::new(media_item.name.clone())
                            .centered()
                            .render(episode, buf);
                    }
                    _ => {
                        panic!("Unexpected media item type: {media_item:#?}");
                    }
                }
            } else {
                Paragraph::new("Nowthing is currently playing").render(main, buf);
            }
            block.render(area, buf);
        } else {
            Paragraph::new("Waiting for player state").render(main, buf);
        }
        Ok(())
    }
}
