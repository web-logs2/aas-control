#[macro_use]
extern crate diesel;
extern crate dotenv;

use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};

use crate::session::SessionMap;
use api::*;
use cleanup::cleanup;
use std::path::Path;
use tokio::time::{sleep, Duration};

pub mod aas;
mod api;
mod cleanup;
pub mod db;
mod session;

// User OSS login related consts
pub const OSS_PROVIDER_URL: &str = "https://passport.openanolis.cn/login";
pub const CALL_BACK_URL: &str = "http%3A%2F%2Fattestation.openanolis.cn%2Fredirect";
pub const USER_INFO_API_URL: &str = "https://epoint.openanolis.cn/common/ucenter/verifyToken.json";
// pub const SERVER_KEY: &str = "456789";
pub const SERVER_KEY: &str = "kArXcY4q";
// pub const AUTH_SIGN_KEY: &str = "b3BlbmNvcmFsLWNu";
pub const TOKEN_COOKIE_ID: &str = "_oc_ut";

// Work Dir
pub const WORK_DIR: &str = "/opt/aas-control";

// AAS instance config file names
pub const KBS_CONFIG_FILE: &str = "kbs-config.json";
pub const AS_CONFIG_FILE: &str = "as-config.json";
pub const QCNL_CONFIG_FILE: &str = "sgx_default_qcnl.conf";

// DataBase URL
pub const DATABASE_URL: &str = "mysql://localhost/aas_control";

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    if !Path::new(WORK_DIR).exists() {
        std::fs::create_dir_all(WORK_DIR)?;
    }

    std::fs::write(
        format!("{WORK_DIR}/{KBS_CONFIG_FILE}"),
        std::include_str!("../../configs/kbs-config.json").to_string(),
    )?;
    std::fs::write(
        format!("{WORK_DIR}/{AS_CONFIG_FILE}"),
        std::include_str!("../../configs/as-config.json").to_string(),
    )?;
    std::fs::write(
        format!("{WORK_DIR}/{QCNL_CONFIG_FILE}"),
        std::include_str!("../../configs/sgx_default_qcnl.conf").to_string(),
    )?;
    std::fs::write(
        format!("{WORK_DIR}/docker-compose.yml"),
        std::include_str!("../../scripts/docker-compose.yml").to_string(),
    )?;
    let cp_client_output = std::process::Command::new("cp")
        .arg("-r")
        .arg("./static/aas_client")
        .arg(format!("{WORK_DIR}/"))
        .output()?;
    if !cp_client_output.status.success() {
        panic!(
            "Copy client static directory failed: {}",
            String::from_utf8_lossy(&cp_client_output.stderr)
        );
    }
    std::fs::write(
        format!("{WORK_DIR}/guide.html"),
        std::include_str!("../../static/guide.html").to_string(),
    )?;
    std::fs::write(
        format!("{WORK_DIR}/invitation_list.conf"),
        std::include_str!("../../static/invitation_list.conf").to_string(),
    )?;

    let sessions = web::Data::new(SessionMap::new());
    let db_connection_pool = db::get_connection_pool().unwrap();

    let _cleanup = tokio::spawn(async {
        loop {
            let db_connection_pool_for_clean = db::get_connection_pool().unwrap();
            match cleanup(db_connection_pool_for_clean).await {
                Err(e) => log::error!("Cleanup spawn failed: {:?}", e),
                Ok(()) => log::info!("Cleanup check done"),
            };
            sleep(Duration::from_secs(5 * 60)).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::clone(&web::Data::new(
                db_connection_pool.clone(),
            )))
            .app_data(web::Data::clone(&sessions))
            .service(start)
            .service(redirect)
            .route("/aas-client-csv", web::get().to(download_aas_client_csv))
            .route("/aas-client-snp", web::get().to(download_aas_client_snp))
            .route(
                "/aas-client-intel",
                web::get().to(download_aas_client_intel),
            )
            .route("/guide", web::get().to(get_guide))
    })
    .bind(("0.0.0.0", 7001))?
    .run()
    .await
}
