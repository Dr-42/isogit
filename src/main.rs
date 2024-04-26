use anyhow::Result;
use axum::{routing, Router};
use std::env;

mod getters;
mod internal;

#[tokio::main]
async fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let mut port = 8080;
    if args.len() == 2 {
        port = args[1].parse::<u16>()?;
    } else {
        println!("Usage: {} <port>", args[0]);
    }

    let ip = format!("0.0.0.0:{}", port);

    let routes = Router::new()
        .route("/", routing::get(getters::home))
        .route("/index.css", routing::get(getters::index_css))
        .route("/images/logo.png", routing::get(getters::logo))
        .route("/index.js", routing::get(getters::index_js))
        .route("/internal/repolist", routing::get(getters::repolist));
    let router_service = routes.into_make_service();
    axum::Server::bind(&ip.parse()?)
        .serve(router_service)
        .await?;
    Ok(())
}
