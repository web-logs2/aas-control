-- Your SQL goes here
CREATE TABLE openanolis_users (
  id bigint PRIMARY KEY auto_increment comment 'ID',
  userno TEXT NOT NULL comment 'no',
  username TEXT NOT NULL comment 'user name',
  email TEXT NOT NULL comment 'user Email',
  aas_auth_key TEXT comment 'AAS instance auth private key',
  aas_instance BOOLEAN NOT NULL DEFAULT 0,
  insert_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
)