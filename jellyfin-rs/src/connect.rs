use std::{
    cell::UnsafeCell,
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    net::IpAddr,
    ops::{Deref, DerefMut},
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Poll, ready},
};

use bytes::{Bytes, BytesMut};
use color_eyre::{Section, SectionExt, eyre::eyre};
use futures_util::{FutureExt, future::poll_fn};
use http::{
    Request, Response,
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    response::Parts,
    uri::Authority,
};
use hyper::{
    body::{Body, Incoming},
    client::conn::http1,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use pin_project_lite::pin_project;
use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::Semaphore,
};
use tokio_rustls::{
    TlsConnector,
    client::TlsStream,
    rustls::{ClientConfig, RootCertStore, pki_types::ServerName},
};
use tracing::{Instrument, error, error_span, info_span, warn};

use crate::Result;

#[derive(Clone)]
pub struct ConnectionConfig {
    pub authority: Authority,
    pub host: ServerName<'static>,
    pub port: u16,
    pub tls: bool,
    pub general_config: TlsConnector,
    pub http1_config: TlsConnector,
}

impl ConnectionConfig {
    pub fn new(authority: Authority, tls: bool) -> Result<Self> {
        let host = ServerName::try_from(authority.host())?.to_owned();
        let port = authority.port_u16().unwrap_or(if tls { 443 } else { 80 });
        let mut cert_store = RootCertStore::empty();
        let certs = rustls_native_certs::load_native_certs();
        if let Some(e) = certs.errors.into_iter().next() {
            return Err(e.into());
        }
        for cert in certs.certs {
            cert_store.add(cert)?
        }
        let cert_store = Arc::new(cert_store);
        let http1_config = ClientConfig::builder()
            .with_root_certificates(cert_store)
            .with_no_client_auth();
        let mut general_config = http1_config.clone();
        general_config.alpn_protocols.push("h2".as_bytes().to_vec());
        general_config
            .alpn_protocols
            .push("http/1.1".as_bytes().to_vec());
        Ok(ConnectionConfig {
            authority,
            host,
            port,
            tls,
            general_config: Arc::new(general_config).into(),
            http1_config: Arc::new(http1_config).into(),
        })
    }

    pub async fn http1_base_connection(&self) -> Result<MaybeTls> {
        let stream = get_stream(&self.host, self.port).await?;
        let stream = if self.tls {
            MaybeTls::Tcp {
                stream: self.http1_config.connect(self.host.clone(), stream).await?,
            }
        } else {
            MaybeTls::Plain { stream }
        };
        Ok(stream)
    }

    async fn connection(&self) -> Result<ConnectionInner> {
        let stream = get_stream(&self.host, self.port).await?;
        Ok(if self.tls {
            let stream = self
                .general_config
                .connect(self.host.clone(), stream)
                .await?;
            if let Some(b"h2") = stream.get_ref().1.alpn_protocol() {
                let (send, con) = hyper::client::conn::http2::handshake(
                    TokioExecutor::new(),
                    TokioIo::new(stream),
                )
                .await?;
                spawn_con(con);
                ConnectionInner::H2(send)
            } else {
                let (send, con) = http1::handshake(TokioIo::new(stream)).await?;
                spawn_con(con);
                ConnectionInner::H1(send)
            }
        } else {
            let (send, con) = http1::handshake(TokioIo::new(stream)).await?;
            spawn_con(con);
            ConnectionInner::H1(send)
        })
    }
}

pub struct Connection {
    pub config: ConnectionConfig,
    guard: Semaphore,
    inner: Vec<Exclusive<ConnectionInner>>,
}

struct Exclusive<T> {
    inner: UnsafeCell<T>,
    in_use: AtomicBool,
}

unsafe impl<T: Send> Send for Exclusive<T> {}
unsafe impl<T: Send> Sync for Exclusive<T> {}

impl<T> Exclusive<T> {
    fn new(v: T) -> Self {
        Exclusive {
            inner: UnsafeCell::new(v),
            in_use: AtomicBool::new(false),
        }
    }
    fn get(&self) -> Option<Guard<'_, T>> {
        if self
            .in_use
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(Guard {
                inner: unsafe { &mut *self.inner.get() },
                in_use: &self.in_use,
            })
        } else {
            None
        }
    }
}

struct Guard<'v, T> {
    inner: &'v mut T,
    in_use: &'v AtomicBool,
}

impl<'v, T> Drop for Guard<'v, T> {
    fn drop(&mut self) {
        self.in_use.store(false, Ordering::Release);
    }
}

impl<'v, T> Deref for Guard<'v, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'v, T> DerefMut for Guard<'v, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("config", &self.config)
            .finish()
    }
}

impl Debug for ConnectionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionConfig")
            .field("authority", &self.authority)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("tls", &self.tls)
            .finish()
    }
}

enum ConnectionInner {
    Disconnected,
    H2(hyper::client::conn::http2::SendRequest<String>),
    H1(http1::SendRequest<String>),
}

const MAX_RETRIES: u8 = 4;

impl Connection {
    pub fn new(authority: Authority, tls: bool, concurrency: usize) -> Result<Self> {
        Ok(Self {
            config: ConnectionConfig::new(authority, tls)?,
            guard: Semaphore::const_new(concurrency),
            inner: (0..concurrency)
                .map(|_| Exclusive::new(ConnectionInner::Disconnected))
                .collect(),
        })
    }

    pub(crate) fn with_same_config(&self) -> Self {
        Self {
            config: self.config.clone(),
            guard: Semaphore::const_new(self.inner.len()),
            inner: (0..self.inner.len())
                .map(|_| Exclusive::new(ConnectionInner::Disconnected))
                .collect(),
        }
    }

    pub async fn send_request_json<T: DeserializeOwned>(
        &self,
        req: Request<String>,
    ) -> Result<JsonResponse<T>> {
        let (data, parts) = self.send_request(req).await?;
        if let Some(content_type) = parts.headers.get(CONTENT_TYPE)
            && content_type.to_str()?.contains("application/json")
        {
            Ok(JsonResponse::from(Bytes::from(data)))
        } else {
            Err(eyre!("Response does not have json CONTENT_TYPE"))
        }
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn send_request(&self, req: Request<String>) -> Result<(BytesMut, Parts)> {
        let uri = req.uri().to_string();
        let span = info_span!("send_request", uri);
        async move {
            let permit = self.guard.acquire().await.expect("should never be closed");
            let mut state = 'outer: {
                for s in &self.inner {
                    if let Some(s) = s.get() {
                        break 'outer s;
                    }
                }
                panic!("all states are currently locked")
            };
            let mut retries = 0u8;
            let res = loop {
                if retries > MAX_RETRIES{
                    color_eyre::eyre::bail!("sending request failed after {MAX_RETRIES} retries")
                }
                let resp = loop {
                    let inner = match state.deref_mut() {
                        ConnectionInner::Disconnected => self.config.connection().await?,
                        ConnectionInner::H2(send_request) => {
                            if let Err(e) = send_request.ready().await {
                                error!("error sending request: {e:?}");
                                retries += 1;
                                ConnectionInner::Disconnected
                            } else {
                                break send_request.send_request(req.clone()).left_future();
                            }
                        }
                        ConnectionInner::H1(send_request) => {
                            if let Err(e) = send_request.ready().await {
                                error!("error sending request: {e:?}");
                                retries += 1;
                                ConnectionInner::Disconnected
                            } else {
                                break send_request.send_request(req.clone()).right_future();
                            }
                        }
                    };
                    *state = inner;
                };
                match resp.await {
                    Ok(resp) => break recv_response(check_status(resp)?).await,
                    Err(e) => {
                        warn!("received connection error: {e:?}");
                        retries += 1;
                        warn!("retrying request");
                    }
                }
            };
            drop(state);
            drop(permit);
            res
        }
        .instrument(span)
        .await
    }
}

fn spawn_con(con: impl Future<Output = hyper::Result<()>> + Send + 'static) {
    tokio::spawn(
        async move {
            if let Err(e) = pin!(con).await {
                error!("connection error: {e:?}")
            }
        }
        .instrument(error_span!("jellyfin_connection")),
    );
}

async fn get_stream(host: &ServerName<'static>, port: u16) -> Result<TcpStream> {
    Ok(match host {
        ServerName::DnsName(dns_name) => TcpStream::connect((dns_name.as_ref(), port)).await?,
        ServerName::IpAddress(ip_addr) => {
            TcpStream::connect((IpAddr::from(ip_addr.to_owned()), port)).await?
        }
        _ => unimplemented!(),
    })
}

fn check_status<T>(response: Response<T>) -> Result<Response<T>> {
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        Err(eyre!("HTTP Error encountered: {status}"))
    } else {
        Ok(response)
    }
}

async fn recv_response(response: Response<Incoming>) -> Result<(BytesMut, Parts)> {
    let mut out = if let Some(length) = response.headers().get(CONTENT_LENGTH) {
        let length: usize = length.to_str()?.parse()?;
        BytesMut::with_capacity(length)
    } else {
        BytesMut::new()
    };
    let (parts, body) = response.into_parts();
    let mut parts = Some(parts);
    let mut body = pin!(body);
    poll_fn(move |cx| {
        while !body.is_end_stream() {
            if let Some(frame) = ready!(body.as_mut().poll_frame(cx)) {
                match frame {
                    Ok(frame) => {
                        if let Some(data) = frame.data_ref() {
                            out.extend_from_slice(data.as_ref())
                        }
                    }
                    Err(e) => return Poll::Ready(Err(e.into())),
                }
            } else {
                break;
            }
        }
        Poll::Ready(Ok((
            std::mem::replace(&mut out, BytesMut::new()),
            parts.take().expect("called twice"),
        )))
    })
    .await
}

pub struct JsonResponse<T: DeserializeOwned> {
    response: Bytes,
    deserialize: PhantomData<T>,
}

impl<T: DeserializeOwned> JsonResponse<T> {
    pub fn deserialize(self) -> impl Future<Output = Result<T>> {
        self.deserialize_as::<T>()
    }
    pub async fn deserialize_value(
        self,
    ) -> std::result::Result<serde_json::Value, serde_json::Error> {
        serde_json::from_slice(&self.response)
    }
    pub async fn deserialize_as<V: DeserializeOwned>(self) -> Result<V> {
        match serde_json::from_slice::<V>(&self.response) {
            Ok(v) => Ok(v),
            Err(first_try) => {
                let first_try = color_eyre::Report::from(first_try);
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&self.response) {
                    Err(first_try.wrap_err("Type mismatched with data").section(
                        serde_json::to_string_pretty(&v)
                            .expect("value should always be valid")
                            .header("Value:"),
                    ))
                } else {
                    Err(first_try.wrap_err("Invalid json syntax"))
                }
            }
        }
    }
}

impl<T: DeserializeOwned> From<Bytes> for JsonResponse<T> {
    fn from(value: Bytes) -> Self {
        JsonResponse {
            response: value,
            deserialize: PhantomData,
        }
    }
}

pin_project! {
    #[project = MaybeTlsProj]
    pub enum MaybeTls {
        Plain {
            #[pin]
            stream: TcpStream,
        },
        Tcp {
            #[pin]
            stream: TlsStream<TcpStream>,
        },
    }
}
impl AsyncRead for MaybeTls {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Plain { stream } => stream.poll_read(cx, buf),
            MaybeTlsProj::Tcp { stream } => stream.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MaybeTls {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        match self.project() {
            MaybeTlsProj::Plain { stream } => stream.poll_write(cx, buf),
            MaybeTlsProj::Tcp { stream } => stream.poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        match self.project() {
            MaybeTlsProj::Plain { stream } => stream.poll_flush(cx),
            MaybeTlsProj::Tcp { stream } => stream.poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        match self.project() {
            MaybeTlsProj::Plain { stream } => stream.poll_shutdown(cx),
            MaybeTlsProj::Tcp { stream } => stream.poll_shutdown(cx),
        }
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        match self.project() {
            MaybeTlsProj::Plain { stream } => stream.poll_write_vectored(cx, bufs),
            MaybeTlsProj::Tcp { stream } => stream.poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            MaybeTls::Plain { stream } => stream.is_write_vectored(),
            MaybeTls::Tcp { stream } => stream.is_write_vectored(),
        }
    }
}
