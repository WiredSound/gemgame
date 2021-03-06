mod handling;
mod id;
mod maps;
mod networking;

use std::{path::PathBuf, sync::Arc};

use maps::ServerMap;
use parking_lot::Mutex;
use structopt::StructOpt;
use tokio::{net::TcpListener, sync::broadcast};

/// Create an [`sqlx::query::Query`] instance using the SQL query in the specified file with the `.sql` extension
/// (`server/db/` directory). In a database argument is provided then a query execution future is created.
#[macro_export]
macro_rules! db_query_from_file {
    ($file:expr) => {
        sqlx::query(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/db/", $file, ".sql")))
    };
    ($file:expr, $db:expr) => {
        db_query_from_file!($file).execute($db)
    };
}

#[tokio::main]
async fn main() {
    // Command-line arguments:
    let options = Options::from_args();

    // Logger initialisation:

    let log_level = {
        if options.log_debug {
            flexi_logger::Level::Debug
        }
        else if options.log_trace {
            flexi_logger::Level::Trace
        }
        else {
            flexi_logger::Level::Info
        }
    }
    .to_level_filter();

    let mut log_spec_builder = flexi_logger::LogSpecBuilder::new();
    log_spec_builder.default(log_level);

    for module in &["sqlx", "tungstenite", "tokio_tungstenite", "mio"] {
        log_spec_builder.module(module, flexi_logger::LevelFilter::Warn);
    }

    let log_spec = log_spec_builder.finalize();

    let mut logger = flexi_logger::Logger::with(log_spec)
        .log_target(flexi_logger::LogTarget::StdOut)
        .format_for_stdout(flexi_logger::colored_detailed_format);

    if options.log_to_file {
        logger = logger
            .log_target(flexi_logger::LogTarget::File)
            .format_for_files(flexi_logger::detailed_format)
            .duplicate_to_stdout(flexi_logger::Duplicate::All)
            .rotate(
                flexi_logger::Criterion::Age(flexi_logger::Age::Day),
                flexi_logger::Naming::Timestamps,
                flexi_logger::Cleanup::KeepLogFiles(3)
            );
    }
    logger.start().expect("Failed to initialise logger");

    // Bind socket and handle connections:

    let host_address = format!("0.0.0.0:{}", options.port);

    let listener = TcpListener::bind(&host_address).await.expect("Failed to create TCP/IP listener");
    log::info!("Created TCP/IP listener bound to address: {}", host_address);

    // Connect to database:

    let db_pool_options = sqlx::postgres::PgPoolOptions::new().max_connections(options.max_database_connections);
    let db_pool =
        db_pool_options.connect(&options.database_connection_string).await.expect("Failed to connect to database");

    log::info!(
        "Created connection pool with maximum of {} simultaneous connections to database",
        options.max_database_connections
    );

    db_query_from_file!("client_entities/create table", &db_pool).await.unwrap();
    db_query_from_file!("map/create table", &db_pool).await.unwrap();
    db_query_from_file!("map_chunks/create table", &db_pool).await.unwrap();

    log::info!("Prepared necessary database tables");

    // Load/create game map that is to be shared between threads:

    let contained_map = ServerMap::load_or_new(&db_pool).await.unwrap();
    let map: Shared<ServerMap> = Arc::new(Mutex::new(contained_map));
    log::info!("Prepared game map");

    // Create multi-producer, multi-consumer channel so that each task may notify every other task of changes made to
    // the game world:

    let (map_changes_sender, mut map_changes_receiver) = broadcast::channel(5);

    log::info!("Listening for incoming TCP/IP connections...");

    loop {
        // Connections will be continuously listened for unless Ctrl-C is pressed and the loop is exited. Messages on
        // the world modifcation channel are also listened for and immediately discarded. This is done as the main task
        // must maintain access to the channel in order to clone and pass it to new connection tasks while also not
        // blocking the broadcasted message queue.
        tokio::select!(
            res = listener.accept() => {
                let (stream, address) = res.unwrap();

                log::info!("Incoming connection from: {}", address);

                tokio::spawn(handling::handle_connection(
                    stream,
                    address,
                    Arc::clone(&map),
                    db_pool.clone(),
                    map_changes_sender.clone(),
                    map_changes_sender.subscribe()
                ));
            }
            _ = map_changes_receiver.recv() => {} // Discard the broadcasted world modification message.
            _ = tokio::signal::ctrl_c() => break // Break on Ctrl-C.
        );
    }

    log::info!("No longer listening for connections");
}

/// Alias for a [`Mutex`] wrapped in an [`Arc`].
type Shared<T> = Arc<Mutex<T>>;

/// Server application for GemGame.
#[derive(StructOpt, Debug)]
#[structopt(name = "GemGame Server")]
struct Options {
    /// The port on which listen for incoming connections.
    #[structopt(short, long, default_value = "5678")]
    port: u16,

    /// Directory containing game map data.
    #[structopt(long, default_value = "map/", parse(from_os_str))]
    map_directory: PathBuf,

    /// Specify how to connect to the database.
    #[structopt(long, default_value = "postgres://localhost/gemgame")]
    database_connection_string: String,

    /// Specify the maximum number of connections that the database connection pool is able to have open
    /// simultaneously.
    #[structopt(long, default_value = "25")]
    max_database_connections: u32,

    /// Display all debugging logger messages.
    #[structopt(long, conflicts_with = "log-trace")]
    log_debug: bool,

    /// Display all tracing and debugging logger messages.
    #[structopt(long)]
    log_trace: bool,

    /// Specifiy whether or not log messages should be written to a file in addition to stdout.
    #[structopt(long)]
    log_to_file: bool
}
