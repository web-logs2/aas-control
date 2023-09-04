# Anolis-AS Control Service

# Deploy

```shell
# rm -rf /etc/app_log.txt
nohup ./target/release/aas-control-service >/etc/aas.log 2>&1 &
```