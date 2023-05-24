use actix_web::cookie::{
    time::{Duration, OffsetDateTime},
    Cookie, Expiration,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

pub const SESSION_ID: &str = "aas-create-session-id";
const SESSION_TIMEOUT: i64 = 5;

pub struct Session<'a> {
    cookie: Cookie<'a>,
    userno: Option<String>,
}

impl<'a> Session<'a> {
    pub fn new() -> Self {
        let id = Uuid::new_v4().as_simple().to_string();
        let cookie = Cookie::build(SESSION_ID, id)
            .expires(OffsetDateTime::now_utc() + Duration::minutes(SESSION_TIMEOUT))
            .finish();

        Self {
            cookie,
            userno: None,
        }
    }

    pub fn id(&self) -> &str {
        self.cookie.value()
    }

    pub fn cookie(&self) -> Cookie {
        self.cookie.clone()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(Expiration::DateTime(time)) = self.cookie.expires() {
            return OffsetDateTime::now_utc() > time;
        }

        false
    }

    pub fn set_user_no(&mut self, userno: String) {
        self.userno = Some(userno)
    }
}

pub struct SessionMap<'a> {
    pub sessions: RwLock<HashMap<String, Arc<Mutex<Session<'a>>>>>,
}

impl<'a> SessionMap<'a> {
    pub fn new() -> Self {
        SessionMap {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}
