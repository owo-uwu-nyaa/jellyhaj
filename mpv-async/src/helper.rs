use crate::{Mpv, Result};

pub trait MpvExt {
    fn set_pause(&self, pause: bool) -> Result<()>;
    fn set_fullscreen(&self, pause: bool) -> Result<()>;
}

impl MpvExt for Mpv {
    fn set_pause(&self, pause: bool) -> Result<()> {
        self.set_property(c"pause", pause)?;
        Ok(())
    }

    fn set_fullscreen(&self, fullscreen: bool) -> Result<()> {
        self.set_property(c"fullscreen", fullscreen)?;
        Ok(())
    }
}
