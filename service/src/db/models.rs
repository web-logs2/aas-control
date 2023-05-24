use super::schema::openanolis_users;
use serde::Deserialize;

#[derive(Insertable, Clone, Debug, Deserialize)]
#[diesel(table_name = openanolis_users)]
pub struct NewUser {
    pub userno: String,
    pub username: String,
    pub email: String,
}

#[derive(Queryable, Debug)]
pub struct OpenAnolisUser {
    #[diesel(sql_type = Bigint)]
    pub id: i64,
    #[diesel(sql_type = Text)]
    pub userno: String,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Text)]
    pub aas_auth_key: Option<String>,
    pub aas_instance: bool,
    #[diesel(sql_type = Timestamp)]
    pub insert_time: time::PrimitiveDateTime,
}
