# OpenAnolis远程证明服务配置与测试

## 先决条件
1. 登录OpenAnolis账号并获取OpenAnolis远程证明服务 (AAS) 实例，若还未创建，请点击[这里](http://aas-control.openanolis.cn).
在创建实例完成后，会获得AAS实例的URL地址，本文档中以下述URL地址为例：
```
http://aas-control.openanolis.cn/1234567
```
创建完成后还会获得一个认证密钥，请将密钥写入本地文件备用，例如:
```
cat>/etc/aas-auth.key<<EOF
-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIPxQzuqDpOTq2KuMAdQhZjmAegDVrjCKk0DnibQQqS1e
-----END PRIVATE KEY-----
EOF
```

2. 下载并安装AAS-Client命令行工具，龙蜥社区为OpenAnolis OS提供了rpm安装包，请点击[这里](http://aas-control.openanolis.cn/aas-client)下载.

## Attestation

在一个真实可信硬件的TEE中运行client，进行attestation测试：

```
aas-client --url http://aas-control.openanolis.cn/1234567 attest
```
请注意将url参数替换为您的AAS实例URL.

这行命令将会在TEE搜集远程证明证据，并向AAS发起认证请求，在请求通过后，返回一个包含了证据验证结果且被签名的JWT令牌，而证据验证结果中包含了从证据中解析出的表征TEE可信度的重要内容，例如硬件安全版本号，软件度量值等等。

**注：**若暂时没有真实硬件上的TEE环境，可以设置如下环境变量后再运行aas-client，以使用Sample TEE类型来测试AAS的Attestation功能：

```
export AA_SAMPLE_ATTESTER_TEST=yes
```

### 获取令牌验证公钥
通过如下命令从AAS获取用于验证令牌签名的公钥：
```
curl -k -X GET http://aas-control.openanolis.cn/1234567/kbs/v0/token-certificate-chain
```
这会返回一个JWKS格式的公钥，并包含自签名的x.509格式证书。

### 设置自定义Attestation策略 (可选)

可以通过如下命令设置自定义的Attestation策略（rego语法），以在硬件厂商要求的基本的默认证据验证策略之外，附加上所需要的进一步的证据内容检查：
```
aas-client config --auth-private-key /etc/aas-auth.key set-attestation-policy --type rego --id default --policy-file /path/to/policy.rego
```

请注意将`/etc/aas-auth.key`替换为您的认证密钥，将`/path/to/policy.rego`替换为策略文件的路径.

若不设置自定义策略，则AAS不会在硬件厂商默认策略之外做更多的证据验证检查。