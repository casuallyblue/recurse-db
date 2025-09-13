use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    routing,
};
use std::{collections::HashMap, fs::File, io::BufRead, path::Path};
use std::{io::BufReader, sync::Mutex};
use std::{io::Write, sync::Arc};

struct DBState {
    kv_store: HashMap<String, String>,
    storage: File,
}

#[tokio::main]
async fn main() {
    let storage_path = Path::new("db.dat");

    let storage = File::options()
        .read(true)
        .append(true)
        .create(true)
        .open(storage_path)
        .unwrap();

    let reader = BufReader::new(storage.try_clone().unwrap());

    let mut kv_store = HashMap::new();

    for line in reader.lines() {
        if let Ok(line) = line {
            let mut results = line.split(30 as char);

            kv_store.insert(
                results.next().unwrap().to_string(),
                results.next().unwrap().to_string(),
            );
        }
    }

    let db_state = Arc::new(Mutex::new(DBState { kv_store, storage }));

    let db = Router::new()
        .route("/set", routing::get(set))
        .route("/get", routing::get(get))
        .with_state(db_state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(listener, db)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    let db_state = db_state.lock().unwrap();

    let mut storage_file = File::create("db.dat.new").unwrap();

    for (key, value) in db_state.kv_store.iter() {
        storage_file.write(key.as_bytes()).unwrap();
        storage_file.write(&[30]).unwrap();
        storage_file.write(value.as_bytes()).unwrap();
        storage_file.write("\n".as_bytes()).unwrap();
    }

    drop(storage_file);
    drop(db_state);

    std::fs::rename("db.dat", "db.dat.old").unwrap();
    std::fs::rename("db.dat.new", "db.dat").unwrap();
    std::fs::remove_file("db.dat.old").unwrap();

    println!("server stopped");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    tokio::select! {
        _ = ctrl_c => {},
    }
}

async fn get(
    Query(params): Query<HashMap<String, String>>,
    State(db_state): State<Arc<Mutex<DBState>>>,
) -> Result<String, StatusCode> {
    if let Some(key_name) = params.get("key") {
        let db_state = db_state.lock().unwrap();

        if let Some(key_value) = db_state.kv_store.get(key_name) {
            return Ok(key_value.clone());
        }
    }

    Err(StatusCode::NOT_FOUND)
}

async fn set(
    Query(params): Query<HashMap<String, String>>,
    State(db_state): State<Arc<Mutex<DBState>>>,
) -> StatusCode {
    if params.len() != 1 {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    for (key, value) in params {
        let mut db_state = db_state.lock().unwrap();
        db_state.kv_store.insert(key.clone(), value.clone());

        db_state.storage.write(key.as_bytes()).unwrap();
        db_state.storage.write(&[30]).unwrap();
        db_state.storage.write(value.as_bytes()).unwrap();
        db_state.storage.write("\n".as_bytes()).unwrap();

        return StatusCode::OK;
    }

    // The for loop will execute exactly once since
    // we checked that the number of query parameters
    // was equal to one. Since it returns there, it
    // should be impossible to reach this point.
    unreachable!("should not be possible to get here");
}
