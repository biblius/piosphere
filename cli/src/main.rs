use std::io::stdin;

use piteria::{
    socket::{client::Client, PiteriaMessage},
    PITERIA_SOCKET,
};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};

#[tokio::main]
async fn main() {
    println!("Starting client");
    let client = Client::new(PITERIA_SOCKET)
        .await
        .expect("Could not connect to Piteria server");

    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap();
    let res = client
        .request(PiteriaMessage::ViewDeployment(5))
        .await
        .expect("error in request");
    println!("Got response: {:?}", res);

    let mut signals = Signals::new([SIGTERM, SIGINT]).unwrap();
    let signals = tokio::spawn(async move {
        for sig in signals.forever() {
            println!("Received signal {:?}", sig);

            if sig == SIGINT || sig == SIGTERM {
                let result = client.close().await;
                return result;
            }
        }
        unreachable!()
    });

    let _ = signals.await.expect("error while shutting down");
}
