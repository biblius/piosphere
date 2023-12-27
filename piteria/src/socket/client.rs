use crate::{
    socket::{read, write},
    PiteriaMessage,
};
use tokio::{
    net::UnixStream,
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::{PiteriaIOError, PiteriaIOResult, PiteriaRequest, PiteriaResponse};

pub struct Client {
    tx: Sender<PiteriaRequest>,
    session_handle: JoinHandle<()>,
    terminate_tx: Sender<()>,
}

impl Client {
    pub async fn new(socket: &str) -> PiteriaIOResult<Self> {
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

        this.request(PiteriaMessage::Hello).await?;

        println!("Client successfully initialized");

        Ok(this)
    }

    /// Send a Piteria message to the server and wait for a response.
    pub async fn request(&self, msg: PiteriaMessage) -> PiteriaIOResult<PiteriaResponse> {
        println!("Client requesting: {:?}", msg);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let request = PiteriaRequest { tx, msg };

        if let Err(e) = self.tx.send(request).await {
            println!("Error while sending to session: {e}");
            return Err(PiteriaIOError::ChannelClosed(e.to_string()));
        }

        let res = rx.await?;

        println!("Client got: {res:?}");

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
    msg_rx: Receiver<PiteriaRequest>,
}

impl ClientSession {
    fn new(
        stream: UnixStream,
        terminate_rx: Receiver<()>,
        msg_rx: Receiver<PiteriaRequest>,
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
                        let PiteriaRequest { tx, msg } = msg;
                        println!("Client sending: {:?}", msg);
                        if let Err(PiteriaIOError::Io(e)) =
                            write(&mut self.stream, msg).await
                        {
                            println!("Error occurred while writing to socket: {e}");
                            continue;
                        }
                        let response = read(&mut self.stream).await;
                        match response {
                            Ok(res) => {
                                println!("Session got response: {:?}", res);
                                if tx.send(res).is_err() {
                                    println!("Could not forward response to client")
                                }
                            }
                            Err(e) => {
                                println!("Error while reading: {e}");
                                if let PiteriaIOError::SocketClosed(msg) = e {
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
}
