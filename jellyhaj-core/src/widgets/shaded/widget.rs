use std::{
    cmp::max,
    fmt::Debug,
    ops::{Deref, DerefMut},
    time::Duration,
};

use config::{Config, effects::EffectInfo};
use jellyhaj_widgets_core::{Buffer, ContextRef, GetFromContext, JellyhajWidget, Rect, Result};
use spawn::Spawner;
use tokio::time::Instant;

use crate::{
    state::Navigation,
    widgets::{ErasedWidget, KeybindAction, erased},
};

pub type Erased = Box<ShadedWidget<Navigation>>;

#[must_use]
pub struct ShadedWidgetGen<W: ?Sized> {
    last: Instant,
    start: Option<EffectInfo>,
    main: Option<EffectInfo>,
    exit: Option<EffectInfo>,
    widget: W,
}

pub type ShadedWidget<Res> = ShadedWidgetGen<dyn ErasedWidget<Res>>;

pub fn make_new_erased<
    R: ContextRef<Spawner> + ContextRef<Config> + Send + 'static,
    A: Debug + Send + 'static,
    W: JellyhajWidget<R, Action = KeybindAction<A>>,
>(
    cx: R,
    widget: W,
) -> Box<ShadedWidget<W::ActionResult>> {
    ShadedWidget::new(cx, widget)
}

impl<Res: 'static> ShadedWidget<Res> {
    pub fn new_sized<
        R: ContextRef<Spawner> + ContextRef<Config> + Send + 'static,
        A: Debug + Send + 'static,
        W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Res>,
    >(
        cx: R,
        widget: W,
    ) -> ShadedWidgetGen<impl ErasedWidget<Res>> {
        let effects = &Config::get_ref(&cx).effects;
        let start = effects.start(W::NAME);
        let main = effects.main(W::NAME);
        let exit = effects.exit(W::NAME);
        let widget = erased::make_new_erased(cx, widget);
        ShadedWidgetGen {
            last: Instant::now(),
            start,
            main,
            exit,
            widget,
        }
    }
    pub fn new<
        R: ContextRef<Spawner> + ContextRef<Config> + Send + 'static,
        A: Debug + Send + 'static,
        W: JellyhajWidget<R, Action = KeybindAction<A>, ActionResult = Res>,
    >(
        cx: R,
        widget: W,
    ) -> Box<Self> {
        Box::new(Self::new_sized(cx, widget))
    }

    pub fn is_stopped_finished(&self) -> bool {
        self.exit.is_none()
    }

    pub fn render_shaded(&mut self, area: Rect, buf: &mut Buffer) -> Result<u8> {
        self.render(area, buf)?;
        let now = Instant::now();
        let time = now - self.last;
        self.last = now;
        let mut fps = 0u8;
        if let Some(main) = self.main.as_mut() {
            main.effect.process(time, buf, area);
            if main.effect.done() {
                self.main = None;
            } else {
                fps = main.fps;
            }
        }
        if let Some(start) = self.start.as_mut() {
            start.effect.process(time, buf, area);
            if start.effect.done() {
                self.start = None;
            } else {
                fps = max(fps, start.fps);
            }
        }
        Ok(fps)
    }
    pub fn start_render_stop(&mut self, area: Rect, buf: &mut Buffer) -> Result<u8> {
        self.render(area, buf)?;
        let now = Instant::now();
        let time = now - self.last;
        self.last = now;
        let mut fps = 0u8;
        if let Some(main) = self.main.as_mut() {
            main.effect.process(time, buf, area);
            if main.effect.done() {
                self.main = None;
            } else {
                fps = main.fps;
            }
        }
        if let Some(start) = self.start.as_mut() {
            start.effect.process(time, buf, area);
            if start.effect.done() {
                self.start = None;
            } else {
                fps = max(fps, start.fps);
            }
        }
        if let Some(exit) = self.exit.as_mut() {
            exit.effect.process(Duration::ZERO, buf, area);
            if exit.effect.done() {
                self.exit = None;
            } else {
                fps = max(fps, exit.fps);
            }
        }
        Ok(fps)
    }
    pub fn render_stop(&mut self, area: Rect, buf: &mut Buffer) -> Result<u8> {
        self.render(area, buf)?;
        let now = Instant::now();
        let time = now - self.last;
        self.last = now;
        let mut fps = 0u8;
        if let Some(main) = self.main.as_mut() {
            main.effect.process(time, buf, area);
            if main.effect.done() {
                self.main = None;
            } else {
                fps = main.fps;
            }
        }
        if let Some(start) = self.start.as_mut() {
            start.effect.process(time, buf, area);
            if start.effect.done() {
                self.start = None;
            } else {
                fps = max(fps, start.fps);
            }
        }
        if let Some(exit) = self.exit.as_mut() {
            exit.effect.process(time, buf, area);
            if exit.effect.done() {
                self.exit = None;
            } else {
                fps = max(fps, exit.fps);
            }
        }
        Ok(fps)
    }
}

impl<Res> Deref for ShadedWidget<Res> {
    type Target = dyn ErasedWidget<Res>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<Res> DerefMut for ShadedWidget<Res> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}
