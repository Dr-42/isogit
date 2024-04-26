use std::path::Path;

use anyhow::Result;
use axum::http::Response;

pub struct RepoDetails {
    pub name: String,
    pub description: String,
}

pub async fn repolist() -> Result<Response<String>> {
    if !Path::new("repo-details.json").exists() {
        Ok(Response::new("[]".to_string()))
    } else {
        let contents = async_fs::read_to_string("repo-details.json").await?;
        Ok(Response::new(contents))
    }
}
