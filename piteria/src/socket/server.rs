use crate::{
    socket::{Header, PiteriaIOError, PiteriaRequest, HEADER_SIZE},
    PiteriaResult, PiteriaService,
};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, io::ErrorKind, path::Path, sync::Arc};
use tokio::{
    io::AsyncReadExt,
    net::{UnixListener, UnixStream},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::PiteriaIOResult;

pub struct Server {
    terminate_tx: Sender<()>,
    rt_handle: JoinHandle<()>,
}

impl Server {
    pub fn new(service: PiteriaService, socket: &str) -> Self {
        let socket = Path::new(socket);

        // Delete old socket if necessary
        if socket.exists() {
            std::fs::remove_file(socket).unwrap();
        }

        println!("Binding to {}", socket.display());
        let listener = UnixListener::bind(socket).unwrap();

        let (terminate_tx, terminate_rx) = tokio::sync::mpsc::channel(128);
        let (sys_tx, sys_rx) = tokio::sync::mpsc::channel(128);

        let rt = ServerRuntime::new(listener, sys_rx, terminate_rx, Arc::new(service));

        let handle = rt.run(sys_tx);

        Self {
            terminate_tx,
            rt_handle: handle,
        }
    }

    pub async fn close(self) -> Result<(), tokio::task::JoinError> {
        self.terminate_tx.send(()).await.unwrap();
        println!("Sent termination to runtime");
        self.rt_handle.await
    }
}

#[derive(Debug)]
struct ServerRuntime {
    terminate_rx: Receiver<()>,
    listener: UnixListener,
    sys_rx: Receiver<SystemMessage>,
    terminators: HashMap<usize, Sender<()>>,
    handles: HashMap<usize, JoinHandle<()>>,
    next_id: usize,
    service: Arc<PiteriaService>,
}

impl ServerRuntime {
    fn new(
        listener: UnixListener,
        sys_rx: Receiver<SystemMessage>,
        terminate_rx: Receiver<()>,
        service: Arc<PiteriaService>,
    ) -> Self {
        Self {
            terminate_rx,
            listener,
            sys_rx,
            terminators: HashMap::new(),
            handles: HashMap::new(),
            next_id: 0,
            service,
        }
    }

    fn run(mut self, sys_tx: Sender<SystemMessage>) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {

                    // Accept new connections

                    res = self.listener.accept() => {
                        match res {
                            Ok((socket, addr)) => {
                                println!("Got a client: {:?} - {:?}", socket, addr);

                                let (term_tx, term_rx) = tokio::sync::mpsc::channel(128);
                                let session_id = self.gen_id();
                                let session = ServerSession {
                                    id: session_id,
                                    stream: socket,
                                    sys_tx: sys_tx.clone(),
                                    terminate_rx: term_rx,
                                    service: self.service.clone(),
                                };
                                let handle = session.run();
                                self.terminators.insert(session_id, term_tx);
                                self.handles.insert(session_id, handle);
                            }
                            Err(e) => println!("Error while accepting connection: {:?}", e),
                        }
                    }

                    msg = self.sys_rx.recv() => {
                        println!("Runtime handling sys message: {:?}", msg);
                        if let Some(msg) = msg {
                            if let Err(e) = self.process_sys(msg).await {
                                println!("Error while processing system message: {e}");
                            }
                        } else {
                            println!("Runtime system receiver has no senders, stopping");
                            break;
                        }
                    }

                    // Terminate server if necessary

                    _ = self.terminate_rx.recv() => {
                        println!("Runtime terminating");

                        for (id, term) in self.terminators.into_iter() {
                            println!("Sending termination to {id}");
                            if let Err(e) = term.send(()).await {
                                println!("Error while terminating session: {e}");
                            }
                        }

                        for (id, handle) in self.handles.into_iter() {
                            if let Err(e) = handle.await {
                                println!("Error while joining session {id}: {e}");
                            }
                        }

                        break;
                    }
                }
            }
        })
    }

    async fn process_sys(&mut self, message: SystemMessage) -> PiteriaResult<()> {
        match message {
            SystemMessage::Close(id) => {
                let handle = self.handles.remove(&id);
                if let Some(handle) = handle {
                    let _ = handle.await;
                }
                self.terminators.remove(&id);
            }
        }
        Ok(())
    }

    fn gen_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.overflowing_add(1).0;
        id
    }
}

struct ServerSession {
    id: usize,

    /// Unix socket handle
    stream: UnixStream,

    /// Sending end for system messages
    sys_tx: Sender<SystemMessage>,

    /// Termination receiver
    terminate_rx: Receiver<()>,

    service: Arc<PiteriaService>,
}

impl ServerSession {
    fn run(mut self) -> JoinHandle<()> {
        println!("Spawning session");
        tokio::spawn(async move {
            loop {
                tokio::select! {

                message = Self::read::<PiteriaRequest>(&mut self.stream) => {
                        println!("Session got message: {:?}", message);
                        match message {
                            Ok(message) => {
                                self.service.respond(&mut self.stream, message).await.unwrap();
                            }
                            Err(e) => {
                                match e {
                                    PiteriaIOError::SocketClosed(msg) => {
                                        println!("Socket closed: {msg}, terminating session");
                                        self.sys_tx.send(SystemMessage::Close(self.id)).await.unwrap();
                                        break;
                                    },
                                    _ => dbg!(e),
                                };
                            }
                        }
                }

                _ = self.terminate_rx.recv() => {
                    println!("Session terminating");
                    break;
                }
                }
            }
        })
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

        let mut buf = vec![0; len];
        stream.read_exact(&mut buf).await?;

        let msg = bincode::deserialize(&buf)?;

        Ok(msg)
    }
}

#[derive(Debug)]
enum SystemMessage {
    /// Sent when a session closes
    Close(usize),
}
