use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};

use crate::session::SessionMap;
use api::{redirect, start};

mod api;
mod session;

pub const OIDC_CLIENT_ID: &str = "client_id=d11a2e3a5a9c4fb4340e";
pub const OIDC_CLIENT_SECRET: &str = "client_secret=47d05d22400b128b43819e3147f272e83f758b1b";
pub const OIDC_TOKEN_SCOPE: &str = "scope=read:user";

pub const ACCESS_CODE_QUREY_PREFIX: &str = "code=";

pub const OIDC_PROVIDER_URL: &str = "https://github.com/login/oauth/authorize";
pub const OIDC_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const USER_INFO_URL: &str = "https://api.github.com/user";

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let sessions = web::Data::new(SessionMap::new());

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::clone(&sessions))
            .service(start)
            .service(redirect)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
