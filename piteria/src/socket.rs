//! Exposes main functionality for unix sockets to be used by the server and clients.

use macros::request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::array::TryFromSliceError;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, net::UnixStream};

use crate::{deployment::Deployment, PiteriaResult};

pub mod client;
pub mod server;

type PiteriaIOResult<T> = Result<T, PiteriaIOError>;

const HEADER_SIZE: usize = std::mem::size_of::<usize>();

type PiteriaHeader = [u8; HEADER_SIZE];

pub trait Message: Serialize {
    type Response: DeserializeOwned;

    fn to_request(&self) -> PiteriaResult<PiteriaRequest> {
        let tag = self.tag();
        let message = bincode::serialize(self)?;
        let req = PiteriaRequest { tag, message };
        Ok(req)
    }

    fn tag(&self) -> PiteriaTag;
}

#[derive(Debug, Serialize, Deserialize)]
#[request(Self, Hello)]
pub struct Hello;

#[derive(Debug, Serialize, Deserialize)]
#[request(Vec<crate::db::Deployment>, Overview)]
pub struct Overview;

#[derive(Debug, Serialize, Deserialize)]
#[request(Vec<Deployment>, ViewDeployment)]
pub struct ViewDeployment(pub i64);

#[derive(Debug, Serialize, Deserialize)]
pub struct PiteriaRequest {
    pub tag: PiteriaTag,
    pub message: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PiteriaTag {
    Hello,
    Overview,
    ViewDeployment,
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

#[allow(async_fn_in_trait)]
pub trait PiteriaWrite {
    async fn write<T: Serialize>(&mut self, message: T) -> PiteriaIOResult<()>;
}

impl PiteriaWrite for UnixStream {
    async fn write<T: Serialize>(&mut self, message: T) -> PiteriaIOResult<()> {
        self.writable().await?;

        println!("Stream is writable");
        let request = bincode::serialize(&message)?;

        let header = Header::create(request.len());

        self.write_all(&header).await?;
        println!("Wrote header");

        self.write_all(&request).await?;
        println!("Wrote body");

        self.flush().await?;
        println!("Socket Flushed");

        Ok(())
    }
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
