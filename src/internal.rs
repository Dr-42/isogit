use axum::extract::Query;
use axum::http::StatusCode;
use axum::{extract::Json, http::Response};
use axum_macros::debug_handler;
use git2::TreeEntry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Serialize, Deserialize)]
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

#[debug_handler]
pub async fn add_repo(Json(payload): Json<Value>) -> Response<String> {
    if !Path::new("repos").exists() {
        let res = async_fs::create_dir("repos").await;
        if res.is_err() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Unable to create repos directory".to_string())
                .unwrap();
        }
    }
    if let Ok(repo) = serde_json::from_value::<RepoDetails>(payload) {
        let repo_path = format!("repos/{}.git", repo.name);
        if !Path::new(&repo_path).exists() {
            let res = git2::Repository::init_bare(&repo_path);
            if res.is_err() {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Unable to create repository".to_string())
                    .unwrap();
            }
        }
        if !Path::new("repo-details.json").exists() {
            let repo_details = serde_json::to_string(&vec![repo]);
            if repo_details.is_err() {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Unable to serialize repo details".to_string())
                    .unwrap();
            }
            let repo_details = repo_details.unwrap();
            let write_op = async_fs::write("repo-details.json", repo_details).await;
            if write_op.is_err() {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Unable to write repo details".to_string())
                    .unwrap();
            }
            return Response::new("".to_string());
        }
        let file_contents = async_fs::read_to_string("repo-details.json").await;
        if file_contents.is_err() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Unable to read repo details".to_string())
                .unwrap();
        }
        let file_contents = file_contents.unwrap();
        let prev_contents = serde_json::from_str(&file_contents);
        if prev_contents.is_err() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Unable to deserialize repo details".to_string())
                .unwrap();
        }
        let mut prev_contents: Vec<RepoDetails> = prev_contents.unwrap();
        prev_contents.push(repo);
        let repo_details = serde_json::to_string(&prev_contents);
        if repo_details.is_err() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Unable to serialize repo details".to_string())
                .unwrap();
        }
        let repo_details = repo_details.unwrap();
        let res = async_fs::write("repo-details.json", repo_details).await;
        if res.is_err() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Unable to write repo details".to_string())
                .unwrap();
        }
        Response::builder()
            .status(StatusCode::CREATED)
            .body("".to_string())
            .unwrap()
    } else {
        Response::new("Invalid JSON".to_string())
    }
}

#[derive(Serialize)]
struct CommitDetails {
    commit_id: String,
    files: Vec<FileDetails>,
}

#[derive(Serialize)]
struct FileDetails {
    name: String,
    tree_id: String,
}

fn tree_recur(
    entry: TreeEntry,
    repo: &git2::Repository,
    parent_name: Option<String>,
) -> Vec<FileDetails> {
    let mut files: Vec<FileDetails> = Vec::new();
    if entry.kind() == Some(git2::ObjectType::Tree) {
        let tree = repo.find_tree(entry.id()).unwrap();
        for tree_entry in tree.iter() {
            if tree_entry.kind() == Some(git2::ObjectType::Tree) {
                let mut name = tree_entry.name().unwrap().to_string();
                if let Some(parent_name) = &parent_name {
                    name = format!("{}/{}", parent_name, name);
                }
                let mut children = tree_recur(tree_entry, repo, Some(name));
                files.append(&mut children);
            } else {
                let name = tree_entry.name().unwrap().to_string();
                let name = if let Some(parent_name) = &parent_name {
                    format!("{}/{}", parent_name, name)
                } else {
                    name
                };
                let tree_id = tree_entry.id().to_string();
                files.push(FileDetails { name, tree_id });
            }
        }
    }
    files
}

#[debug_handler]
pub async fn get_filelist(Query(query): Query<HashMap<String, String>>) -> Response<String> {
    let name = query.get("name").unwrap();
    let repo_path = format!("repos/{}.git", name);
    let repo_path = Path::new(&repo_path);

    if !repo_path.exists() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Repository not found".to_string())
            .unwrap();
    }
    let repo = git2::Repository::open(repo_path);
    if repo.is_err() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Unable to open repository".to_string())
            .unwrap();
    }

    let repo = repo.unwrap();
    let mut walker = repo.revwalk().unwrap();
    walker.push_head().unwrap();
    let mut commits: Vec<CommitDetails> = Vec::new();
    for oid in walker {
        let oid = oid.unwrap();
        let tree = repo.find_commit(oid).unwrap().tree().unwrap();
        let commit_id = oid.to_string();
        let mut files: Vec<FileDetails> = Vec::new();
        for entry in tree.iter() {
            if entry.kind() == Some(git2::ObjectType::Tree) {
                let name = entry.name().unwrap().to_string();
                let mut children = tree_recur(entry, &repo, Some(name));
                files.append(&mut children);
            } else {
                let name = entry.name().unwrap().to_string();
                let tree_id = entry.id().to_string();
                files.push(FileDetails { name, tree_id });
            }
        }
        commits.push(CommitDetails { commit_id, files });
    }
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&commits).unwrap())
        .unwrap()
}
