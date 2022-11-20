use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
};

use askama::Template;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use hyper::server::{accept::Accept, conn::AddrIncoming};
use status::Status;
use tracing::{info, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{Config, ListenStack};

mod config;
mod status;

#[tokio::main]
async fn main() {
    let config_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.kdl".to_string());

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "mine_status=info,tower_http=error".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::init(config_file).await.unwrap();
    info!("config: {:?}", config);

    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), config.listen_port);
    let incoming_v4 = AddrIncoming::bind(&localhost_v4).unwrap();

    let localhost_v6 = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), config.listen_port);
    let incoming_v6 = AddrIncoming::bind(&localhost_v6).unwrap();

    // build our application with a route
    let services = config.services.clone();
    let app = Router::new()
        .route("/", get(move || get_status(services)))
        .route("/ip", get(get_ip));

    // add a fallback service for handling routes to unknown paths
    let app = app.fallback(handler_404);

    // run it
    if let ListenStack::Both = config.listen_stack {
        info!("listening v4 on http://{}", &localhost_v4);
        info!("listening v6 on http://{}", &localhost_v6);
        let combined = CombinedIncoming {
            a: incoming_v4,
            b: incoming_v6,
        };
        axum::Server::builder(combined)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    } else {
        let incoming = match config.listen_stack {
            config::ListenStack::V4 => {
                info!("listening v4 on http://{}", &localhost_v4);
                incoming_v4
            }
            config::ListenStack::V6 => {
                info!("listening v6 on http://{}", &localhost_v6);
                incoming_v6
            }
            _ => unreachable!(),
        };

        axum::Server::builder(incoming)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    }
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

async fn get_status(services: Vec<String>) -> impl IntoResponse {
    let status = Status::init(services).await;
    HtmlTemplate(status)
}

async fn get_ip(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(?req);
    trace!(?addr);
    let req_ip_str = addr.ip().to_string();
    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .map(|i| i.to_str().unwrap())
        .unwrap_or(&req_ip_str);
    ip.to_string()
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

struct CombinedIncoming {
    a: AddrIncoming,
    b: AddrIncoming,
}

impl Accept for CombinedIncoming {
    type Conn = <AddrIncoming as Accept>::Conn;
    type Error = <AddrIncoming as Accept>::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        if let Poll::Ready(Some(value)) = Pin::new(&mut self.a).poll_accept(cx) {
            return Poll::Ready(Some(value));
        }

        if let Poll::Ready(Some(value)) = Pin::new(&mut self.b).poll_accept(cx) {
            return Poll::Ready(Some(value));
        }

        Poll::Pending
    }
}
