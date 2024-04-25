use axum::{
    body::{self, Full},
    http::{header, Response},
    response::{Html, IntoResponse},
};

pub async fn home() -> Html<String> {
    let code = include_str!("web/index.html");
    Html(code.to_string())
}

pub async fn index_css() -> impl IntoResponse {
    let code = include_str!("web/index.css");
    axum::http::Response::builder()
        .header("Content-Type", "text/css")
        .body(code.to_string())
        .unwrap()
}

pub async fn logo() -> impl IntoResponse {
    let m = "image/x-icon";
    let body = include_bytes!("web/images/logo.png").to_vec();
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_str(m).unwrap(),
        )
        .body(body::boxed(Full::from(body)))
        .unwrap()
}
