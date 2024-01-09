//! Exposes main functionality for unix sockets to be used by the server and clients.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::array::TryFromSliceError;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use crate::PiosphereResult;

pub mod client;
pub mod message;
pub mod server;

type PiosphereIOResult<T> = Result<T, PiosphereIOError>;

const HEADER_SIZE: usize = std::mem::size_of::<usize>();

type PiosphereHeader = [u8; HEADER_SIZE];

pub(crate) trait Header: Sized {
    fn size(&self) -> usize;

    fn create(size: usize) -> Self;

    async fn read(stream: &mut UnixStream) -> PiosphereIOResult<Self>;
}

impl Header for PiosphereHeader {
    fn size(&self) -> usize {
        usize::from_le_bytes(*self)
    }

    fn create(size: usize) -> Self {
        size.to_le_bytes()
    }

    async fn read(stream: &mut UnixStream) -> PiosphereIOResult<Self> {
        let mut buf = [0; HEADER_SIZE];
        stream.read_exact(&mut buf).await?;
        Ok(buf)
    }
}

#[allow(async_fn_in_trait)]
pub trait Message: Serialize + Sized {
    /// A tag identifies
    type Response: DeserializeOwned;

    fn to_request(&self) -> PiosphereResult<PiosphereRequest> {
        let tag = self.tag();
        let message = bincode::serialize(self)?;
        Ok(PiosphereRequest { tag, message })
    }

    fn tag(&self) -> PiosphereTag;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PiosphereRequest {
    pub tag: PiosphereTag,
    pub message: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PiosphereTag {
    Hello,
    Overview,
    ViewDeployment,
}

#[derive(Debug, Error)]
pub enum PiosphereIOError {
    #[error("{0}")]
    SocketClosed(String),

    #[error("{0}")]
    ChannelClosed(String),

    #[error("{0}")]
    Response(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("{0}")]
    Bincode(#[from] bincode::Error),

    #[error("{0}")]
    MalformedHeader(#[from] TryFromSliceError),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[allow(async_fn_in_trait)]
pub trait PiosphereWrite {
    async fn write<T: Serialize>(&mut self, message: T) -> PiosphereIOResult<()>;
}

impl PiosphereWrite for UnixStream {
    async fn write<T: Serialize>(&mut self, message: T) -> PiosphereIOResult<()> {
        self.writable().await?;

        println!("Stream is writable");
        let request = bincode::serialize(&message)?;

        let header = PiosphereHeader::create(request.len());

        self.write_all(&header).await?;
        println!("Wrote header");

        self.write_all(&request).await?;
        println!("Wrote body");

        self.flush().await?;
        println!("Socket Flushed");

        Ok(())
    }
}
