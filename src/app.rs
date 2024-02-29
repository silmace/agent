use crate::handlers::{dns, http, mtr, ping, tcping};
use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::Value;
use socketioxide::extract::{Data, SocketRef};
use socketioxide::SocketIo;

static mut API_KEY: String = String::new();

pub fn create_app(api_key: String) -> Router {
    unsafe { API_KEY = api_key };
    let (layer, io) = SocketIo::new_layer();
    io.ns("/", handler);
    Router::new()
        .route("/", get(index))
        .route("/ping", get(pong_handler))
        .layer(layer)
}

async fn pong_handler(header: header::HeaderMap) -> Response<Body> {
    let auth = header.get("authorization").unwrap().to_str().unwrap();
    let key = auth.split(" ").collect::<Vec<&str>>()[1];
    if key != { unsafe { &API_KEY } } {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("pong"))
            .unwrap()
    }
}

async fn index() -> &'static str {
    "Congratulations! You have successfully started the agent."
}

fn handler(socket: SocketRef, Data(_data): Data<Value>) {
    let auth = socket
        .req_parts()
        .headers
        .get("Authorization")
        .unwrap()
        .to_str()
        .unwrap();
    let key = auth.split(" ").collect::<Vec<&str>>()[1];
    if key != { unsafe { &API_KEY } } {
        socket.emit("error", "unauthorized").unwrap();
        socket.disconnect().unwrap();
        return;
    }

    socket.on(
        "ping",
        |socket: SocketRef, Data::<Value>(data)| async move {
            ping(socket, data).await;
        },
    );

    socket.on(
        "tcping",
        |socket: SocketRef, Data::<Value>(data)| async move {
            tcping(socket, data).await;
        },
    );

    socket.on("dns", |socket: SocketRef, Data::<Value>(data)| async move {
        dns(socket, data).await;
    });

    socket.on("mtr", |socket: SocketRef, Data::<Value>(data)| async move {
        mtr(socket, data).await;
    });

    socket.on(
        "http",
        |socket: SocketRef, Data::<Value>(data)| async move {
            http(socket, data).await;
        },
    );
}
