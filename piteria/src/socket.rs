//! Exposes main functionality for unix sockets to be used by the server and clients.

use macros::PiteriaRequest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{array::TryFromSliceError, io::ErrorKind};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::oneshot,
};

pub mod client;
pub mod server;

type PiteriaIOResult<T> = Result<T, PiteriaIOError>;

const HEADER_SIZE: usize = std::mem::size_of::<usize>();

type PiteriaHeader = [u8; HEADER_SIZE];

#[derive(Debug, Serialize, Deserialize, PiteriaRequest)]
pub enum PiteriaMessage {
    Hello,

    #[response(Vec<crate::db::Deployment>)]
    Overview,

    #[response(crate::deployment::Deployment)]
    ViewDeployment(i64),
}

#[derive(Debug)]
struct PiteriaRequest {
    tx: oneshot::Sender<PiteriaResponse>,
    msg: PiteriaMessage,
}

#[derive(Debug, Error)]
pub enum PiteriaIOError {
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

async fn write<T: Serialize>(stream: &mut UnixStream, message: T) -> PiteriaIOResult<()> {
    stream.writable().await?;

    println!("Stream is writable");
    let message = bincode::serialize(&message)?;
    let header = Header::create(message.len());

    stream.write_all(&header).await?;
    println!("Wrote header");

    stream.write_all(&message).await?;
    println!("Wrote body");

    stream.flush().await?;
    println!("Socket Flushed");

    Ok(())
}

async fn read<T: DeserializeOwned>(stream: &mut UnixStream) -> PiteriaIOResult<T> {
    stream.readable().await?;

    let mut buf = [0; HEADER_SIZE];
    if let Err(e) = stream.read_exact(&mut buf).await {
        if let ErrorKind::UnexpectedEof = e.kind() {
            return Err(PiteriaIOError::SocketClosed(e.to_string()));
        }
    };

    let len = Header::size(buf);
    println!("Read header: {len}");

    let buf = &mut vec![0; len];
    stream.read_exact(buf).await?;

    let msg = bincode::deserialize(buf)?;

    Ok(msg)
}

#[derive(Debug)]
struct Header;

impl Header {
    pub fn create(size: usize) -> PiteriaHeader {
        size.to_le_bytes()
    }

    pub fn size(bytes: [u8; HEADER_SIZE]) -> usize {
        usize::from_le_bytes(bytes)
    }
}
