use crate::aas::create_aas_instance;
use crate::db::{
    models::{NewUser, OpenAnolisUser},
    schema::{openanolis_users, openanolis_users::dsl::*},
    MysqlPool,
};
use crate::session::{Session, SessionMap, SESSION_ID};
use actix_files::NamedFile;
use actix_web::{body::BoxBody, get, web, HttpRequest, HttpResponse};
use anyhow::anyhow;
use anyhow::{bail, Result};
use diesel::prelude::*;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

use crate::cleanup::DURATION_HOURS;
use crate::{SERVER_KEY, TOKEN_COOKIE_ID, USER_INFO_API_URL, WORK_DIR};

const AAS_CLIENT_FILE_CSV: &str = "aas_client_an8_csv_0.1.0.tar.gz";
const AAS_CLIENT_FILE_SNP: &str = "aas_client_an8_snp_0.1.0.tar.gz";
const AAS_CLIENT_FILE_INTEL: &str = "aas_client_an8_intel_0.1.0.tar.gz";

macro_rules! bail_error_internal {
    ($error: expr) => {
        match $error {
            Ok(inner) => inner,
            Err(e) => {
                log::error!("InternalServerError: {:?}", e);
                return HttpResponse::InternalServerError()
                    .message_body(BoxBody::new(e.to_string()))
                    .unwrap();
            }
        }
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
    db_connection_pool: web::Data<MysqlPool>,
    map: web::Data<SessionMap<'_>>,
) -> HttpResponse {
    let cookie = match request.cookie(SESSION_ID) {
        Some(c) => c,
        None => {
            log::error!("Missing AAS-control cookie");
            unauthorized!("Missing AAS-control Cookie");
        }
    };
    let sessions = map.sessions.read().await;
    let locked_session = match sessions.get(cookie.value()) {
        Some(ls) => ls,
        None => {
            log::error!("Invalid AAS-control cookie {}", cookie.value());
            unauthorized!("Invalid AAS-control Cookie");
        }
    };
    let mut session = locked_session.lock().await;
    log::info!("AAS-control Cookie {} redirect", session.id());
    if session.is_expired() {
        log::error!("Expired AAS-control cookie {}", cookie.value());
        unauthorized!("Expired AAS-control Cookie");
    }

    let token = match request.cookie(TOKEN_COOKIE_ID) {
        Some(c) => {
            let cookie_value = c.to_string();
            cookie_value.split_once("=").unwrap().1.to_string()
        }
        None => {
            log::error!("Missing Token cookie");
            unauthorized!("Missing Token Cookie");
        }
    };

    let new_user: NewUser = bail_error_internal!(get_user_info(token).await);
    session.set_user_no(new_user.userno.clone());

    let mut db_connection = bail_error_internal!(db_connection_pool.get());
    if bail_error_internal!(openanolis_users
        .filter(userno.eq(new_user.userno.clone()))
        .load::<OpenAnolisUser>(&mut db_connection))
    .len()
        == 0
    {
        bail_error_internal!(diesel::insert_into(openanolis_users::table)
            .values(new_user.clone())
            .execute(&mut db_connection));
    }

    let user_auth_key = bail_error_internal!(
        create_aas_instance(new_user.userno.clone(), &mut db_connection).await
    );

    let html = std::include_str!("../../static/created.html")
        .replace("${USER_NO}", &new_user.userno)
        .replace("${AUTH_KEY}", &user_auth_key)
        .replace("${DURATION_HOURS}", &DURATION_HOURS.to_string());

    println!("\n{user_auth_key}\n");
    return HttpResponse::Ok()
        .append_header(("content-type", "text/html; charset=utf-8"))
        .body(html.as_bytes().to_owned());
}

async fn get_user_info(token: String) -> Result<NewUser> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();
    let nonce: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();
    let mut digest_handler = Sha256::new();
    let payload = format!("{timestamp}:{nonce}:{SERVER_KEY}");
    digest_handler.update(payload.as_bytes().to_vec());
    let digest = digest_handler.finalize();
    let signature_header_value = base64::encode(digest);

    let http_client = reqwest::Client::new();

    let res = http_client
        .get(USER_INFO_API_URL)
        .header("timestamp", format!("{timestamp}"))
        .header("nonce", nonce)
        .header("signature", signature_header_value)
        .header("authorization", token)
        .send()
        .await?;

    let user_value = match res.status() {
        reqwest::StatusCode::OK => res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| anyhow!("Parse Openanolis User Info response Failed: {:?}", e))?,
        _ => {
            bail!("Request Openanolis User Info Failed, {}", res.text().await?);
        }
    };

    let user_info = json!({
        "userno": user_value["data"]["userProfile"]["no"],
        "username": user_value["data"]["userProfile"]["username"],
        "email": user_value["data"]["userProfile"]["email"],
    });
    let user_info_string = serde_json::to_string(&user_info)?;
    let new_user = serde_json::from_str::<NewUser>(&user_info_string)?;

    Ok(new_user)
}

#[get("/")]
pub(crate) async fn start(map: web::Data<SessionMap<'_>>) -> HttpResponse {
    let session = Session::new();

    let html = std::include_str!("../../static/start.html");

    let response = HttpResponse::Ok()
        .cookie(session.cookie())
        .append_header(("content-type", "text/html; charset=utf-8"))
        .body(html.as_bytes().to_owned());

    map.sessions
        .write()
        .await
        .insert(session.id().to_string(), Arc::new(Mutex::new(session)));

    response
}

pub(crate) async fn download_aas_client_csv() -> NamedFile {
    NamedFile::open(format!("{WORK_DIR}/aas_client/{AAS_CLIENT_FILE_CSV}")).unwrap()
}

pub(crate) async fn download_aas_client_snp() -> NamedFile {
    NamedFile::open(format!("{WORK_DIR}/aas_client/{AAS_CLIENT_FILE_SNP}")).unwrap()
}

pub(crate) async fn download_aas_client_intel() -> NamedFile {
    NamedFile::open(format!("{WORK_DIR}/aas_client/{AAS_CLIENT_FILE_INTEL}")).unwrap()
}

pub(crate) async fn get_guide() -> NamedFile {
    NamedFile::open(format!("{WORK_DIR}/guide.html")).unwrap()
}
