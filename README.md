# Anolis-AS control

```shell
# rm -rf /etc/app_log.txt
nohup ./target/debug/aas-control-service >/etc/aas_control_log.txt 2>&1 &
```