use super::TraceResult;
use crate::{
    connect::MaybeTls,
    socket::{JellyfinMessage, JellyfinMessageInternal},
};
use color_eyre::eyre::{Context, Result};
use futures_core::Stream;
use futures_sink::Sink;
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Poll, ready},
    time::Duration,
};
use tokio::time::Interval;
use tokio_websockets::{Message, WebSocketStream};

pin_project! {
    #[project = PollSocketProj]
    pub(crate) struct PollSocket{
        closed: bool,
       #[pin]
        keep_alive: Option<Interval>,
        send_keep_alive: bool,
       #[pin]
        socket: WebSocketStream<MaybeTls>
    }
}

impl PollSocket {
    pub(crate) const fn new(stream: WebSocketStream<MaybeTls>) -> Self {
        Self {
            closed: false,
            keep_alive: None,
            send_keep_alive: false,
            socket: stream,
        }
    }
}

impl Stream for PollSocket {
    type Item = JellyfinMessage;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        if *this.closed {
            poll_close(cx, this.socket)
        } else {
            loop {
                if let Some(mut keep_alive) = this.keep_alive.as_mut().as_pin_mut()
                    && let Poll::Ready(_) = keep_alive.poll_tick(cx)
                {
                    *this.send_keep_alive = true;
                }
                if *this.send_keep_alive
                    && let Poll::Ready(res) = this.socket.as_mut().poll_ready(cx)
                {
                    if res.context("preparing for sending").trace_err().is_none()
                        || this
                            .socket
                            .as_mut()
                            .start_send(Message::text("{\"MessageType\":\"KeepAlive\"}"))
                            .context("sending keep alive")
                            .trace_err()
                            .is_none()
                    {
                        *this.closed = true;
                        break poll_close(cx, this.socket);
                    }
                    *this.send_keep_alive = false;
                }
                if let Some(message) = ready!(poll_message(cx, this.socket.as_mut())) {
                    break Poll::Ready(Some(match message {
                        JellyfinMessageInternal::KeepAlive => {
                            continue;
                        }
                        JellyfinMessageInternal::ForceKeepAlive { data } => {
                            this.keep_alive
                                .set(Some(tokio::time::interval(Duration::from_secs(data) / 2)));
                            continue;
                        }
                        JellyfinMessageInternal::Public(message) => message,
                    }));
                }
                *this.closed = true;
                break poll_close(cx, this.socket);
            }
        }
    }
}

fn poll_close(
    cx: &mut std::task::Context<'_>,
    socket: Pin<&mut WebSocketStream<MaybeTls>>,
) -> Poll<Option<JellyfinMessage>> {
    ready!(socket.poll_close(cx))
        .context("close errored")
        .trace_err();
    Poll::Ready(None)
}

fn poll_message(
    cx: &mut std::task::Context<'_>,
    mut socket: Pin<&mut WebSocketStream<MaybeTls>>,
) -> Poll<Option<JellyfinMessageInternal>> {
    let res: Option<Result<_>> = loop {
        let message = match ready!(socket.as_mut().poll_next(cx)) {
            None => break None,
            Some(Err(e)) => break Some(Err(e.into())),
            Some(Ok(message)) => message,
        };
        if let Some(message) = message.as_text() {
            break Some(
                match serde_json::from_str::<JellyfinMessageInternal>(message) {
                    Err(e) => Err(e.into()),
                    Ok(m) => Ok(m),
                },
            );
        }
    };
    Poll::Ready(res.and_then(super::TraceResult::trace_err))
}
