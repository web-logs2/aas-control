# Anolis-AS control

## Prepare

### Install Rust

```shell
curl https://sh.rustup.rs -sSf | bash -s -- -y;
source "$HOME/.cargo/env"
```

### Instal MySQL

```shell
yum install mysql-community-server
systemctl start mysqld
mysql -u root -e create

```

### Install diesel

```shell
yum install diesel
export DB_URL=
diesel migration run
```

### Install Docker

```shell
```

# Build

```shell

```

# Deploy

```shell
# rm -rf /etc/app_log.txt
nohup ./target/release/aas-control-service >/etc/aas.log 2>&1 &
```