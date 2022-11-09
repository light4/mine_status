use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use status::Status;

use crate::config::Config;

mod config;
mod status;

#[tokio::main]
async fn main() {
    let config_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.kdl".to_string());
    // build our application with a route
    let config = Config::init(config_file).await.unwrap();
    let config_clone = config.clone();
    let app = Router::new().route("/", get(move || handler(config_clone)));

    // run it
    println!("listening on {}", &config.bind_addr);
    axum::Server::bind(&config.bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
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

async fn handler(config: Config) -> impl IntoResponse {
    let status = Status::init(&config).await;
    HtmlTemplate(status)
}
