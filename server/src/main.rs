use axum::Router;
use std::net::SocketAddr;

mod config;
mod error;
mod routes;

#[tokio::main]
async fn main() {
    // TODO: Load configuration (e.g., from environment variables or file)
    // TODO: Initialize shared state (e.g., database connection pool, storage backend)

    // build our application router
    let app = routes::create_router(); // Assuming create_router is defined in routes.rs

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080)); // Listen on all interfaces, port 8080
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
