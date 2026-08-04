use jellyhaj_core::state::NextScreen;
use player_core::{Command, PlayerHandle, state::SharedPlayerState};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use zbus::interface;

pub struct MediaPlayer2 {
    player: PlayerHandle,
    state: SharedPlayerState,
    widget_sender: UnboundedSender<NextScreen>,
}

impl MediaPlayer2 {
    pub const fn new(
        player: PlayerHandle,
        state: SharedPlayerState,
        widget_sender: UnboundedSender<NextScreen>,
    ) -> Self {
        Self {
            player,
            state,
            widget_sender,
        }
    }
}

#[allow(clippy::needless_pass_by_value, clippy::unused_self)]
#[interface(name = "org.mpris.MediaPlayer2", spawn = false)]
impl MediaPlayer2 {
    const fn raise(&self) {}
    fn quit(&self) {
        info!("mpris asked us to quit");
        let _ = self.widget_sender.send(NextScreen::Exit);
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn can_quit(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn fullscreen(&self) -> bool {
        self.state.lock().fullscreen
    }
    #[zbus(property)]
    fn set_fullscreen(&self, fullscreen: bool) {
        self.player.send(Command::Fullscreen(fullscreen));
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn can_set_fullscreen(&self) -> bool {
        true
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn can_raise(&self) -> bool {
        false
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn has_track_list(&self) -> bool {
        true
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn identity(&self) -> &'static str {
        "Jellyhaj"
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn desktop_entry(&self) -> &'static str {
        "jellyhaj"
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn supported_uri_schemes(&self) -> &'static [&'static str] {
        &["jellyfin-item"]
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn supported_mime_types(&self) -> &'static [&'static str] {
        &[]
    }
}
