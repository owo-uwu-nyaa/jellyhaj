use jellyhaj_core::context::{DB, ImageCache, JellyfinClient, Stats};
use jellyhaj_image::{JellyfinImage, ParsedImage, Picker};
use jellyhaj_tabs_widget::TabContainer;
use jellyhaj_widgets_core::ContextRef;
use std::convert::Infallible;

#[derive(TabContainer)]
#[tab(action_result=Infallible, common_action=ParsedImage,  cx_constr=
      ContextRef<Picker>+
      ContextRef<Stats>+
      ContextRef<JellyfinClient>+
      ContextRef<DB>+
      ContextRef<ImageCache>)]
pub struct Test {
    #[tab = "i1"]
    image_widget1: JellyfinImage,
    #[tab = "i2"]
    image_widget2: JellyfinImage,
}
