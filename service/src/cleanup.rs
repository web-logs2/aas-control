use crate::db::{
    models::OpenAnolisUser,
    schema::{openanolis_users, openanolis_users::dsl::*},
    MysqlPool,
};
use crate::{
    aas::{NGINX_CONFIG_LOCK, NGINX_CONFIG_PATH},
    WORK_DIR,
};
use anyhow::Result;
use diesel::prelude::*;
use std::process::Command;
use std::sync::Arc;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

#[allow(dead_code)]
pub const DURATION_HOURS: i64 = 24;

pub async fn cleanup(db_connection_pool: MysqlPool) -> Result<()> {
    let mut db_connection = db_connection_pool.get()?;

    let now_odt = OffsetDateTime::now_utc();
    let cutoff_time =
        PrimitiveDateTime::new(now_odt.date(), now_odt.time()) - Duration::hours(DURATION_HOURS); // 24 hours ago

    let users_to_cleanup = openanolis_users
        .filter(insert_time.lt(cutoff_time))
        .load::<OpenAnolisUser>(&mut db_connection)?;

    for user in users_to_cleanup {
        // Cleanup location block in nginx config file.
        let nginx_config_lock_clone = Arc::clone(&NGINX_CONFIG_LOCK);
        let nginx_config_lock = nginx_config_lock_clone.lock().await;
        let nginx_config = std::fs::read_to_string(NGINX_CONFIG_PATH)?;
        let target_location = regex::Regex::new(&format!(
            r#"(?s)location /{}/kbs/v0/\s*\{{.*?\}}"#,
            user.userno.clone()
        ))
        .unwrap();
        let location_match = target_location.find(&nginx_config).unwrap();
        let new_nginx_config = nginx_config.replace(location_match.as_str(), "");
        let cleaned_new_nginx_config = new_nginx_config
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(NGINX_CONFIG_PATH, cleaned_new_nginx_config)?;
        drop(nginx_config_lock);

        // Cleanup AAS instance containers
        Command::new("docker")
            .arg("stop")
            .arg(format!("kbs-{}", &user.userno))
            .arg(format!("as-{}", &user.userno))
            .arg(format!("rvps-{}", &user.userno))
            .status()?;
        Command::new("docker")
            .arg("rm")
            .arg(format!("kbs-{}", &user.userno))
            .arg(format!("as-{}", &user.userno))
            .arg(format!("rvps-{}", &user.userno))
            .status()?;

        // Cleanup user workdir
        std::fs::remove_dir_all(format!("{WORK_DIR}/{}", &user.userno))?;

        // Cleanup user entry in database
        diesel::delete(openanolis_users::table.find(user.id)).execute(&mut db_connection)?;
    }

    Ok(())
}
