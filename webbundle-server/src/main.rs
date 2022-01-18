use anyhow::{Context as _, Result};
use chrono::Local;
use headers::{ContentLength, ContentType, HeaderMapExt as _};
use http::{Response, StatusCode};
use hyper::Body;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use structopt::StructOpt;
use tokio::fs;
use tokio::prelude::*;
use warp::path::Peek;
use warp::Filter as _;
use webbundle::{Bundle, Version};

#[derive(StructOpt, Debug)]
struct Cli {
    /// Uses https
    #[structopt(short = "s", long = "https")]
    https: bool,
    #[structopt(short = "p", long = "port", default_value = "8000")]
    port: u16,
    #[structopt(long = "bind-all")]
    /// Bind all interfaces (default: only localhost - "127.0.0.1"),
    bind_all: bool,
}

type AndThenResult<T> = std::result::Result<T, warp::reject::Rejection>;

fn env_logger_init() {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {:5} {}] ({}:{}) {}",
                Local::now().format("%+"),
                buf.default_styled_level(record.level()),
                record.target(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args(),
            )
        })
        .init();
}

#[tokio::main]
async fn main() {
    env_logger_init();
    let args = Cli::from_args();

    let addr = (
        if args.bind_all {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        },
        args.port,
    );

    // webbundle serving.
    // e.g. GET /wbn/foo => Serves a bundle which is dynamically assembled from the files under directory /foo.
    let webbundle_filter = warp::any()
        .and(warp::path::path("wbn"))
        .and(warp::path::param())
        .and_then(|path: String| async move {
            let root = std::env::current_dir().unwrap();
            let base_dir = normalize_path(&root, &path);
            match webbundle_reply(base_dir).await {
                Ok(response) => AndThenResult::Ok(response),
                Err(err) => {
                    log::error!("Internal Server Error: {:?}", err);
                    Ok(internal_server_error())
                }
            }
        });

    // Static file serving.
    let static_file_filter =
        warp::any()
            .and(warp::path::peek())
            .and_then(|path: Peek| async move {
                match static_file_reply(path.as_str()).await {
                    Ok(response) => AndThenResult::Ok(response),
                    Err(err) => {
                        log::error!("Internal Server Error: {:?}", err);
                        Ok(internal_server_error())
                    }
                }
            });

    let route = warp::get()
        .and(webbundle_filter.or(static_file_filter))
        .with(warp::log::custom(|info| {
            log::info!("{} {} {}", info.method(), info.path(), info.status());
        }));

    if args.https {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        warp::serve(route)
            .tls()
            .cert_path(path.join("examples/tls/cert.pem"))
            .key_path(path.join("examples/tls/key.rsa"))
            .run(addr)
            .await;
    } else {
        warp::serve(route).run(addr).await;
    }
}

async fn webbundle_reply(base_dir: impl AsRef<Path>) -> Result<Response<Body>> {
    let bundle = Bundle::builder()
        .version(Version::VersionB2)
        .primary_url("https://example.com".parse()?)
        .exchanges_from_dir(base_dir, "https://example.com/".parse()?)
        .await?
        .build()?;
    let bytes = bundle.encode()?;
    Ok(response_with(
        ContentLength(bytes.len() as u64),
        ContentType::from("application/webbundle".parse::<mime::Mime>()?),
        bytes,
    ))
}

async fn static_file_reply(path: impl AsRef<Path>) -> Result<Response<Body>> {
    let path = path.as_ref();
    log::debug!("static_file_serv: path: {}", path.display());
    if path.components().any(|p| match p {
        Component::Normal(x) => x == ".git",
        _ => false,
    }) {
        log::warn!(".git is forbidden");
        return Ok(not_found());
    }
    let root = std::env::current_dir().unwrap();
    let file_path = normalize_path(&root, path);
    log::debug!("file_path: {}", file_path.display());

    let file_path = match fs::canonicalize(&file_path).await {
        Ok(file_path) => file_path,
        Err(_) => {
            return Ok(not_found());
        }
    };
    anyhow::ensure!(
        file_path.starts_with(&root),
        "file_path does not starts_with root"
    );
    if file_path.is_dir() {
        log::debug!("is_dir: true");
        // Try to serve <directory>/index.html
        let index_html = file_path.join("index.html");
        if index_html.exists() {
            file_reply(index_html).await
        } else {
            // directory listing
            let body = directory_list_files(
                file_path,
                path.to_str().context("path can not be converted to str")?,
            )
            .await?;
            Ok(response_with(
                ContentLength(body.len() as u64),
                content_type_from_ext("html"),
                body.into_bytes(),
            ))
        }
    } else {
        file_reply(file_path).await
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(b"Not Found".as_ref().into())
        .unwrap()
}

fn internal_server_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(b"Internal Server Error".as_ref().into())
        .unwrap()
}

async fn file_reply(filename: impl AsRef<Path>) -> Result<Response<Body>> {
    log::debug!("file_reply");
    let filename = filename.as_ref();
    if let Ok(mut file) = fs::File::open(filename).await {
        let mut body = Vec::new();
        file.read_to_end(&mut body).await?;
        let len = body.len();
        let mut response = Response::new(Body::from(body));
        response
            .headers_mut()
            .typed_insert(ContentLength(len as u64));
        response
            .headers_mut()
            .typed_insert(content_type_from_path(filename));
        Ok(response)
    } else {
        Ok(not_found())
    }
}

async fn directory_list_files(path: impl AsRef<Path>, display_name: &str) -> Result<String> {
    let path = path.as_ref();
    let mut html_text = String::new();
    html_text.push_str("<h1>my-http-server: Directory listing for ");
    html_text.push_str(display_name);
    html_text.push_str("</h1>");
    html_text.push_str("<hr>");
    html_text.push_str("<ul>");
    html_text.push_str(r#"<li><a href="..">..</a>"#);
    let mut read_dir = fs::read_dir(path).await?;
    let mut files = Vec::new();
    while let Some(file) = read_dir.next_entry().await? {
        files.push(file.path());
    }
    files.sort();
    for p in files {
        let link_name = format!(
            "{}{}",
            p.file_name().unwrap().to_str().unwrap(),
            if p.is_dir() { "/" } else { "" }
        );
        html_text.push_str(&format!("<li><a href={}>{}</a>", link_name, link_name));
    }
    html_text.push_str("</ul>");
    html_text.push_str("<hr>");
    Ok(html_text)
}

fn content_type_from_ext(ext: &str) -> ContentType {
    ContentType::from(mime_guess::from_ext(ext).first_or_octet_stream())
}

fn content_type_from_path(path: &Path) -> ContentType {
    ContentType::from(mime_guess::from_path(path).first_or_octet_stream())
}

fn normalize_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    // We can't use "root.push(req.path()" because req.path() contains "/"
    let path = path
        .as_ref()
        .components()
        .fold(PathBuf::new(), |mut result, p| match p {
            Component::Normal(x) => {
                result.push(x);
                result
            }
            Component::ParentDir => {
                log::warn!("path contains: {}", path.as_ref().display());
                result.pop();
                result
            }
            _ => result,
        });
    root.as_ref().join(path)
}

#[test]
fn normalize_path_test() {
    assert_eq!(normalize_path("/etc", "/foo"), PathBuf::from("/etc/foo"));
    assert_eq!(normalize_path("/etc", ".."), PathBuf::from("/etc"));
    assert_eq!(
        normalize_path("/etc", "/abc/../def"),
        PathBuf::from("/etc/def")
    );
}

fn response_with(
    content_length: ContentLength,
    content_type: ContentType,
    body: Vec<u8>,
) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    response.headers_mut().typed_insert(content_length);
    response.headers_mut().typed_insert(content_type);
    response
}
