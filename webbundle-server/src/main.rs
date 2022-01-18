use axum::{
    body::{boxed, Body, BoxBody},
    routing::{get, get_service},
    Router,
};
use headers::{ContentLength, ContentType, HeaderMapExt as _};
use http::{Request, Response, StatusCode};
use structopt::StructOpt;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use url::Url;
use webbundle::{Bundle, Version};

#[derive(StructOpt, Debug)]
struct Cli {
    // TODO: Support https.
    // #[structopt(short = "s", long = "https")]
    // https: bool,
    #[structopt(short = "p", long = "port", default_value = "8000")]
    port: u16,
    #[structopt(long = "bind-all")]
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
    let args = Cli::from_args();

    let app = Router::new()
        .nest("/wbn", get(webbundle_serve))
        .nest(
            "/static",
            get_service(ServeDir::new(".")).handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
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
    tracing::info!("Listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn webbundle_serve(req: Request<Body>) -> Result<Response<BoxBody>, (StatusCode, String)> {
    webbundle_serve_internal(req).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error {}", err),
        )
    })
}

// TODO: Return an status code
async fn webbundle_serve_internal(req: Request<Body>) -> anyhow::Result<Response<BoxBody>> {
    let path = req.uri().path();
    let mut full_path = std::path::PathBuf::from(".");
    for seg in path.trim_start_matches('/').split('/') {
        anyhow::ensure!(
            !seg.starts_with("..") && !seg.contains('\\'),
            "Invalid request"
        );
        full_path.push(seg);
    }
    anyhow::ensure!(is_dir(&full_path).await, "Not found");

    // TODO: Use relative URL when a relative URL is supported in upstream.
    let base_url: Url = "https://example.com/"
        .parse::<Url>()?
        .join(full_path.to_str().unwrap_or(""))?;

    let bundle = Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir(full_path, base_url)
        .await?
        .build()?;
    let bytes = bundle.encode()?;
    Ok(response_with(
        ContentLength(bytes.len() as u64),
        ContentType::from("application/webbundle".parse::<mime::Mime>()?),
        bytes,
    ))
}

async fn is_dir(full_path: &std::path::Path) -> bool {
    tokio::fs::metadata(full_path)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

fn response_with(
    content_length: ContentLength,
    content_type: ContentType,
    body: Vec<u8>,
) -> Response<BoxBody> {
    let mut response = Response::new(boxed(Body::from(body)));
    response.headers_mut().typed_insert(content_length);
    response.headers_mut().typed_insert(content_type);
    response
}
