use crate::db::{models::OpenAnolisUser, schema::openanolis_users::dsl::*};
use crate::WORK_DIR;
use anyhow::{anyhow, bail, Result};
use diesel::prelude::*;
use diesel::MysqlConnection;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::{AS_CONFIG_FILE, KBS_CONFIG_FILE, QCNL_CONFIG_FILE};

const USER_AUTH_KEY: &str = "private.key";
const USER_PUBKEY: &str = "public.pub";

pub const NGINX_CONFIG_PATH: &str = "/etc/nginx/nginx.conf";
lazy_static::lazy_static! {
    pub static ref NGINX_CONFIG_LOCK: Arc<tokio::sync::Mutex<()>> =
        Arc::new(tokio::sync::Mutex::new(()));
}

pub async fn create_aas_instance(
    user_no: String,
    db_connection: &mut MysqlConnection,
) -> Result<String> {
    // Check whether this user's AAS instance has been created before
    let check = check_created(&user_no, db_connection).await?;
    if check.0 {
        return Ok(check.1.unwrap());
    }

    // Create work dir for this user
    let user_work_dir = format!("{WORK_DIR}/{user_no}");
    if !Path::new(&user_work_dir).exists() {
        fs::create_dir_all(&user_work_dir)
            .map_err(|e| anyhow!("Create User dir failed: {:?}", e))?;
    }

    // Generate Auth Key for this user
    let user_auth_key = generate_auth_key(&user_work_dir)?;

    // Generate AAS config files for this user
    generate_config_files(user_no.clone())?;

    // Update nginx config file to register new AAS instance
    update_nginx_config(user_no.clone()).await?;

    // Start AAS instance for this user
    Command::new("docker-compose")
        .current_dir(format!("{WORK_DIR}/{user_no}"))
        .args(["up", "-d"])
        .status()?;

    // Mark that the AAS instance has been created for this user
    diesel::update(openanolis_users.filter(userno.eq(user_no.clone())))
        .set(aas_auth_key.eq(user_auth_key.clone()))
        .execute(db_connection)?;
    diesel::update(openanolis_users.filter(userno.eq(user_no)))
        .set(aas_instance.eq(true))
        .execute(db_connection)?;

    Ok(user_auth_key)
}

async fn update_nginx_config(user_no: String) -> Result<()> {
    let lock_clone = Arc::clone(&NGINX_CONFIG_LOCK);
    let lock = lock_clone.lock().await;

    let config = std::fs::read_to_string(NGINX_CONFIG_PATH)?;

    let re_server = regex::Regex::new(r"(?s)server\s*\{.*?\}").unwrap();
    let server_match = re_server.find(&config).unwrap();
    let server_block = server_match.as_str();

    let env_file_path = format!("{WORK_DIR}/{user_no}/.env");
    let env_variables = env_file_reader::read_file(env_file_path)?;
    let new_location = format!(
        "
            location /{}/kbs/v0/ {{
                rewrite ^/{}(.*)$ $$1 break;
                proxy_pass http://127.0.0.1:{}/kbs/v0/;
                proxy_set_header Host $$host;
            }}",
        &user_no, &user_no, &env_variables["KBS_PORT"]
    );
    let modified_server_block = format!("{}{}", server_block, new_location);

    let new_config = re_server.replace(&config, modified_server_block);
    fs::write(NGINX_CONFIG_PATH, new_config.to_string())?;
    drop(lock);

    Command::new("nginx").args(["-s", "reload"]).status()?;
    Ok(())
}

fn generate_config_files(user_no: String) -> Result<()> {
    let kbs_port = std::net::TcpListener::bind("0.0.0.0:0")?
        .local_addr()?
        .port();
    let as_port = std::net::TcpListener::bind("0.0.0.0:0")?
        .local_addr()?
        .port();
    let rvps_port = std::net::TcpListener::bind("0.0.0.0:0")?
        .local_addr()?
        .port();

    let env =
        format!("KBS_PORT={kbs_port}\nAS_PORT={as_port}\nRVPS_PORT={rvps_port}\nUSER_NO={user_no}");
    fs::write(format!("{WORK_DIR}/{user_no}/.env"), env)?;

    let kbs_config = fs::read_to_string(format!("{WORK_DIR}/{KBS_CONFIG_FILE}"))?
        .replace("${AS_PORT}", &as_port.to_string());
    let as_config = fs::read_to_string(format!("{WORK_DIR}/{AS_CONFIG_FILE}"))?;
    let sgx_qcnl_config = fs::read_to_string(format!("{WORK_DIR}/{QCNL_CONFIG_FILE}"))?;
    let docker_compose = fs::read_to_string(format!("{WORK_DIR}/docker-compose.yml"))?
        .replace("${USER_NO}", &user_no);

    fs::write(
        format!("{WORK_DIR}/{user_no}/{KBS_CONFIG_FILE}"),
        kbs_config,
    )?;
    fs::write(format!("{WORK_DIR}/{user_no}/{AS_CONFIG_FILE}"), as_config)?;
    fs::write(
        format!("{WORK_DIR}/{user_no}/{QCNL_CONFIG_FILE}"),
        sgx_qcnl_config,
    )?;
    fs::write(
        format!("{WORK_DIR}/{user_no}/docker-compose.yml"),
        docker_compose,
    )?;

    Ok(())
}

// Generate User Auth Key and return the private key
fn generate_auth_key(user_work_dir: &str) -> Result<String> {
    let user_auth_key_path = format!("{user_work_dir}/{USER_AUTH_KEY}");
    let user_pubkey_path = format!("{user_work_dir}/{USER_PUBKEY}");

    let auth_key_file = File::create(&user_auth_key_path)?;
    let genkey_errors = auth_key_file.try_clone()?;
    Command::new("openssl")
        .args(["genpkey", "-algorithm", "ed25519"])
        .stdout(Stdio::from(auth_key_file))
        .stderr(Stdio::from(genkey_errors))
        .spawn()?
        .wait_with_output()?;
    Command::new("openssl")
        .args([
            "pkey",
            "-in",
            &user_auth_key_path,
            "-pubout",
            "-out",
            &user_pubkey_path,
        ])
        .output()?;

    let user_auth_key = fs::read_to_string(user_auth_key_path)
        .map_err(|e| anyhow!("Read User Auth Key failed: {:?}", e))?;
    Ok(user_auth_key)
}

// Check if the AAS instance of this user has been created before,
// if so, return the auth key of this user's AAS instance.
async fn check_created(
    user_no: &str,
    db_connection: &mut MysqlConnection,
) -> Result<(bool, Option<String>)> {
    let user = openanolis_users
        .filter(userno.eq(user_no.to_string()))
        .load::<OpenAnolisUser>(db_connection)?;

    if user.len() == 0 {
        bail!("Cannot find the User in DB");
    } else if user.len() == 1 {
        if user[0].aas_instance && user[0].aas_auth_key.is_some() {
            return Ok((true, user[0].aas_auth_key.clone()));
        } else {
            return Ok((false, None));
        }
    } else {
        bail!("Internal Error: Bad user");
    }
}
