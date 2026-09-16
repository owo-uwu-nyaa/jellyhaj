use std::{
    ops::{Deref, DerefMut},
    task::{Context, Poll},
};

use crate::{Mpv, MpvEvent, Result};

pub struct EventStream<'s, T, F: FnMut(Result<MpvEvent<'_>>) -> T> {
    pub(crate) client: &'s mut Mpv,
    pub(crate) mapper: F,
    pub(crate) exit: bool,
}

impl<T, F: FnMut(Result<MpvEvent<'_>>) -> T> futures_core::Stream for EventStream<'_, T, F> {
    type Item = T;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.exit {
            return Poll::Ready(None);
        }
        let res = std::task::ready!(unsafe { this.client.unsafe_poll_wait_event(cx) });
        if matches!(&res, Ok(MpvEvent::Shutdown)) {
            this.exit = true;
        }
        Poll::Ready(Some((this.mapper)(res)))
    }
}

impl<T, F: FnMut(Result<MpvEvent<'_>>) -> T> DerefMut for EventStream<'_, T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.client
    }
}

impl<T, F: FnMut(Result<MpvEvent<'_>>) -> T> Deref for EventStream<'_, T, F> {
    type Target = Mpv;

    fn deref(&self) -> &Self::Target {
        self.client
    }
}
