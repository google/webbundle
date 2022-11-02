use axum::{
    body::{boxed, Body, BoxBody},
    response::{Html, IntoResponse},
    routing::{get, get_service},
    Router,
};
use axum_extra::middleware::{self, Next};
use clap::Parser;
use headers::{ContentLength, HeaderMapExt as _};
use http::{header, HeaderValue, Request, Response, StatusCode};
use std::fmt::Write as _;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use webbundle::{Bundle, Version};

#[derive(Parser, Debug)]
struct Cli {
    // TODO: Support https.
    // #[arg]
    // https: bool,
    #[arg(short, long, default_value = "8000")]
    port: u16,
    #[arg(long)]
    /// Bind all interfaces (default: only localhost - "127.0.0.1"),
    bind_all: bool,
}

#[tokio::main]
async fn main() {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "my_http_server=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();
    let args = Cli::parse();

    let app = Router::new()
        .nest("/wbn", get(webbundle_serve))
        .fallback(
            get_service(ServeDir::new("."))
                .handle_error(|error: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                })
                .layer(middleware::from_fn(serve_dir_extra)),
        )
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let addr = std::net::SocketAddr::from((
        if args.bind_all {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        },
        args.port,
    ));
    println!("Listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn webbundle_serve(req: Request<Body>) -> Result<Response<BoxBody>, (StatusCode, String)> {
    match webbundle_serve_internal(req).await {
        Ok(WebBundleServeResponse::Body(body)) => Ok(body),
        Ok(WebBundleServeResponse::NotFound) => Err((StatusCode::NOT_FOUND, "".to_string())),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error {}", err),
        )),
    }
}

enum WebBundleServeResponse {
    Body(Response<BoxBody>),
    NotFound,
}

async fn webbundle_serve_internal(req: Request<Body>) -> anyhow::Result<WebBundleServeResponse> {
    let path = req.uri().path();
    let mut full_path = std::path::PathBuf::from(".");
    for seg in path.trim_start_matches('/').split('/') {
        anyhow::ensure!(
            !seg.starts_with("..") && !seg.contains('\\'),
            "Invalid request"
        );
        full_path.push(seg);
    }
    if !is_dir(&full_path).await {
        return Ok(WebBundleServeResponse::NotFound);
    }

    let bundle = Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir(full_path)
        .await?
        .build()?;

    let bytes = bundle.encode()?;
    let content_length = ContentLength(bytes.len() as u64);
    let mut response = Response::new(boxed(Body::from(bytes)));
    response.headers_mut().typed_insert(content_length);
    set_response_webbundle_headers(&mut response);
    Ok(WebBundleServeResponse::Body(response))
}

fn set_response_webbundle_headers(response: &mut Response<BoxBody>) {
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/webbundle"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
}

async fn is_dir(full_path: &std::path::Path) -> bool {
    tokio::fs::metadata(full_path)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

async fn serve_dir_extra(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<Response<BoxBody>, (StatusCode, String)> {
    serve_dir_extra_internal(req, next).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error {}", err),
        )
    })
}

async fn serve_dir_extra_internal(
    req: Request<Body>,
    next: Next<Body>,
) -> anyhow::Result<Response<BoxBody>> {
    // Directory listing.
    // Ref: https://docs.rs/tower-http/0.1.0/src/tower_http/services/fs/serve_dir.rs.html
    let path = req.uri().path();
    let mut full_path = std::path::PathBuf::from(".");
    for seg in path.trim_start_matches('/').split('/') {
        anyhow::ensure!(!seg.starts_with("..") && !seg.contains('\\'));
        full_path.push(seg);
    }
    if is_dir(&full_path).await {
        let html = directory_list_files(full_path, path).await?;
        return Ok(Html(html).into_response());
    }

    if req.uri().path().ends_with(".wbn") {
        let mut res = next.run(req).await;
        set_response_webbundle_headers(&mut res);
        return Ok(res);
    }

    // default.
    Ok(next.run(req).await)
}

async fn directory_list_files(
    path: impl AsRef<std::path::Path>,
    display_name: &str,
) -> anyhow::Result<String> {
    let path = path.as_ref();

    let mut contents = String::new();
    // ReadDir is Stream
    let mut read_dir = tokio::fs::read_dir(path).await?;
    let mut files = Vec::new();
    while let Some(file) = read_dir.next_entry().await? {
        files.push(file.path());
    }
    files.sort();
    for p in files {
        let link_name = format!(
            "{}{}",
            p.file_name().unwrap().to_str().unwrap(),
            if is_dir(&p).await { "/" } else { "" }
        );
        write!(
            contents,
            "<li><a href={link}>{link}</a></li>",
            link = link_name
        )?;
    }

    let inline_style = r#"
body {
  box-sizing: border-box;
  min-width: 200px;
  max-width: 980px;
  margin: 0 auto;
  padding: 45px;
}
"#;

    Ok(format!(
        r#"
<html>
<head><meta charset="utf-8"/>
<title>{title}</title>
<link rel=stylesheet href="https://cdn.jsdelivr.net/npm/github-markdown-css">
<style>
{inline_style}
</style>
</head>
<body class=markdown-body>
<h1>webbundle-server: Directory listing for {display_name}</h1>
<ul>
<li><a href="..">..</a></li>
{contents}
</ul>
<hr>
</body>
</html>
"#,
        title = display_name,
        inline_style = inline_style,
        display_name = display_name,
        contents = contents
    ))
}
