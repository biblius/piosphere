use clap::Parser;
use piteria::{
    db::PiteriaDatabase, socket::server::Server, PiteriaService, PITERIA_DB_FILE, PITERIA_SOCKET,
};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};

#[tokio::main]
async fn main() {
    let args = StartArgs::parse();

    //let db = PiteriaDatabase::new(&args.db).await.unwrap(); // TODO
    let db = PiteriaDatabase::new("piteria.db").await.unwrap();

    println!("Running migrations");

    db.migrate().await.expect("error in migrations");

    println!("Migrations successful");

    let service = PiteriaService::new(db);

    println!("Starting server");

    let mut signals = Signals::new([SIGTERM, SIGINT]).unwrap();

    let handle = Server::new(service, &args.socket);

    let signals = tokio::spawn(async move {
        for sig in signals.forever() {
            println!("Received signal {:?}", sig);

            if sig == SIGINT || sig == SIGTERM {
                println!("Terminating server");
                let result = handle.close().await;
                return result;
            }
        }
        unreachable!()
    });

    println!("Server up and running");

    // Should theoretically never happen since the signals task cannot panic
    signals
        .await
        .expect("error while shutting down")
        .expect("error while shutting down")
}

#[derive(Debug, Parser)]
struct StartArgs {
    #[arg(short, default_value=PITERIA_DB_FILE)]
    db: String,

    #[arg(short, default_value=PITERIA_SOCKET)]
    socket: String,
}
