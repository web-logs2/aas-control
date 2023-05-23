use crate::session::{Session, SessionMap, SESSION_ID};
use actix_web::{body::BoxBody, get, http, web, HttpRequest, HttpResponse};
use anyhow::anyhow;
use anyhow::{bail, Result};
use reqwest::header::HeaderName;
use reqwest::redirect::Policy;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::{
    ACCESS_CODE_QUREY_PREFIX, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET, OIDC_PROVIDER_URL,
    OIDC_TOKEN_SCOPE, OIDC_TOKEN_URL, USER_INFO_URL,
};

macro_rules! bail_error_internal {
    ($error: expr) => {
        match $error {
            Ok(inner) => inner,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .message_body(BoxBody::new(e.to_string()))
                    .unwrap()
            }
        }
    };
}

macro_rules! internal {
    ($reason: expr) => {
        return HttpResponse::InternalServerError()
            .message_body(BoxBody::new($reason))
            .unwrap()
    };
}

macro_rules! unauthorized {
    ($reason: expr) => {
        return HttpResponse::Unauthorized()
            .message_body(BoxBody::new($reason))
            .unwrap()
    };
}

#[get("/redirect")]
pub(crate) async fn redirect(
    request: HttpRequest,
    info: web::Query<serde_json::Value>,
    map: web::Data<SessionMap<'_>>,
) -> HttpResponse {
    let cookie = match request.cookie(SESSION_ID) {
        Some(c) => c,
        None => {
            log::error!("Missing KBS cookie");
            unauthorized!("Missing Cookie");
        }
    };
    let sessions = map.sessions.read().await;
    let locked_session = match sessions.get(cookie.value()) {
        Some(ls) => ls,
        None => {
            log::error!("Invalid cookie {}", cookie.value());
            unauthorized!("Invalid Cookie");
        }
    };
    let mut session = locked_session.lock().await;
    log::info!("Cookie {} redirect", session.id());
    if session.is_expired() {
        log::error!("Expired cookie {}", cookie.value());
        unauthorized!("Expired Cookie");
    }

    let access_code = &info["code"];
    match access_code {
        Value::String(code) => {
            let user_info = bail_error_internal!(get_user_info(code.clone()).await);
            session.set_user_info(user_info.clone());
            let html = format!(
                "<p> OIDC Login Successfully! <br /> User Info: {} </p>",
                user_info
            );
            return HttpResponse::Ok()
                .set_header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(html.as_bytes().to_owned());
        }
        _ => {
            internal!("/redirect must have access_code string qurey");
        }
    }
}

async fn get_user_info(code: String) -> Result<String> {
    let http_client = reqwest::Client::new();

    // Get Access-Token with access-code
    let token_res = http_client
        .post(format!(
            "{OIDC_TOKEN_URL}?{OIDC_CLIENT_ID}&{OIDC_CLIENT_SECRET}&{ACCESS_CODE_QUREY_PREFIX}{code}"
        ))
        .header("accept", "application/json")
        .send()
        .await?;
    let token_value = &token_res.json::<serde_json::Value>().await?["access_token"];
    let token = token_value
        .as_str()
        .ok_or_else(|| anyhow!("No access token in response"))?;

    // Get User Info with Access-Token
    let user_info = http_client
        .get(USER_INFO_URL)
        .header("accept", "application/json")
        .header("user-agent", "curl/7.81.0")
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await?
        .text()
        .await?;
    log::info!("Login success, User Info:\n {}", &user_info);
    Ok(user_info)
}

#[get("/start")]
pub(crate) async fn start(map: web::Data<SessionMap<'_>>) -> HttpResponse {
    let session = Session::new();

    let oidc_url = format!("{OIDC_PROVIDER_URL}?{OIDC_CLIENT_ID}&{OIDC_TOKEN_SCOPE}");
    let html = format!("<a href=\"{oidc_url}\">Start to test github OIDC login</a>");

    let response = HttpResponse::Ok()
        .cookie(session.cookie())
        .set_header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(html.as_bytes().to_owned());

    map.sessions
        .write()
        .await
        .insert(session.id().to_string(), Arc::new(Mutex::new(session)));

    response
}
