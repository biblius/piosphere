use crate::{
    socket::{message::Hello, Header, PiosphereHeader, PiosphereWrite},
    PiosphereResult,
};
use tokio::{
    io::AsyncReadExt,
    net::UnixStream,
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::{Message, PiosphereIOError, PiosphereIOResult, PiosphereRequest};

pub struct Client {
    tx: Sender<PiosphereClientRequest>,
    session_handle: JoinHandle<()>,
    terminate_tx: Sender<()>,
}

impl Client {
    pub async fn new(socket: &str) -> PiosphereResult<Self> {
        let (client_tx, session_rx) = tokio::sync::mpsc::channel(128);
        let (terminate_tx, terminate_rx) = tokio::sync::mpsc::channel(128);

        let stream = UnixStream::connect(socket).await?;

        let session = ClientSession::new(stream, terminate_rx, session_rx);
        let session_handle = session.start();

        let this = Self {
            tx: client_tx,
            session_handle,
            terminate_tx,
        };

        this.request(Hello).await?;

        println!("Client successfully initialized");

        Ok(this)
    }

    /// Send a Piosphere message to the server and wait for a response.
    pub async fn request<M: Message>(&self, msg: M) -> PiosphereResult<M::Response> {
        let request = msg.to_request()?;

        let (rx, request) = PiosphereClientRequest::from_request(request);

        if let Err(e) = self.tx.send(request).await {
            println!("Error while sending to session: {e}");
            return Err(PiosphereIOError::ChannelClosed(e.to_string()).into());
        }

        let res = rx
            .await
            .map_err(|e| PiosphereIOError::ChannelClosed(e.to_string()))?;

        let res = bincode::deserialize(&res)?;

        Ok(res)
    }

    pub async fn close(self) -> Result<(), tokio::task::JoinError> {
        if let Err(e) = self.terminate_tx.send(()).await {
            println!("Error while terminating session: {e}")
        }
        self.session_handle.await
    }
}

struct ClientSession {
    stream: UnixStream,
    terminate_rx: Receiver<()>,
    msg_rx: Receiver<PiosphereClientRequest>,
}

impl ClientSession {
    fn new(
        stream: UnixStream,
        terminate_rx: Receiver<()>,
        msg_rx: Receiver<PiosphereClientRequest>,
    ) -> Self {
        Self {
            stream,
            terminate_rx,
            msg_rx,
        }
    }

    fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {

                    // Terminate client if necessary

                    _ = self.terminate_rx.recv() => {
                        println!("Client terminating");
                        break;
                    }

                    // Send pending messages to the server and wait for a response

                    msg = self.msg_rx.recv() => {
                        let Some(msg) = msg else {
                            continue;
                        };

                        let PiosphereClientRequest { tx, msg } = msg;

                        println!("Client sending: {:?}", msg);

                        if let Err(PiosphereIOError::Io(e)) = self.stream.write(msg).await
                        {
                            println!("Error occurred while writing to socket: {e}");
                            continue;
                        }

                        let response = Self::read(&mut self.stream).await;

                        match response {
                            Ok(res) => {
                                println!("Session got response: {:?}", res);
                                if tx.send(res).is_err() {
                                    println!("Could not forward response to client")
                                }
                            }
                            Err(e) => {
                                println!("Error while reading: {e}");
                                if let PiosphereIOError::SocketClosed(msg) = e {
                                    println!("Socket closed: {msg}, terminating session");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    async fn read(stream: &mut UnixStream) -> PiosphereIOResult<Vec<u8>> {
        stream.readable().await?;

        let header = PiosphereHeader::read(stream).await?;
        let len = header.size();
        println!("Read header: {len}");

        let mut buf = vec![0; len];
        stream.read_exact(&mut buf).await?;

        Ok(buf)
    }
}

/// Intermediary data used by the client and its session to transfer messages
#[derive(Debug)]
struct PiosphereClientRequest {
    tx: oneshot::Sender<Vec<u8>>,
    msg: PiosphereRequest,
}

impl PiosphereClientRequest {
    fn from_request(message: PiosphereRequest) -> (oneshot::Receiver<Vec<u8>>, Self) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let this = Self { tx, msg: message };
        (rx, this)
    }
}
