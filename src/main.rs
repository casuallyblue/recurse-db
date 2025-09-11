use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    routing,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

struct DBState {
    kv_store: HashMap<String, String>,
}

#[tokio::main]
async fn main() {
    let db_state = Arc::new(Mutex::new(DBState {
        kv_store: HashMap::new(),
    }));
    let db = Router::new()
        .route("/set", routing::get(set))
        .route("/get", routing::get(get))
        .with_state(db_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(listener, db)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

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
        db_state.kv_store.insert(key, value.clone());
        return StatusCode::OK;
    }

    // The for loop will execute exactly once since
    // we checked that the number of query parameters
    // was equal to one. Since it returns there, it
    // should be impossible to reach this point.
    unreachable!("should not be possible to get here");
}
