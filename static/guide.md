# OpenAnolis远程证明服务配置与测试

更新日期：2023-6-26

# 先决条件
1. 登录OpenAnolis账号并获取OpenAnolis远程证明服务 (AAS) 实例，若还未创建，请点击[这里](http://attestation.openanolis.cn).
在创建实例完成后，会获得AAS实例的URL地址，本文档中以下述URL地址为例：
```
http://attestation.openanolis.cn/1234567
```
创建完成后还会获得一个认证密钥，请将密钥写入本地文件备用，例如:
```
cat>/etc/aas-auth.key<<EOF
-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIPxQzuqDpOTq2KuMAdQhZjmAegDVrjCKk0DnibQQqS1e
-----END PRIVATE KEY-----
EOF
```

2. 下载并安装AAS-Client命令行工具，龙蜥社区为OpenAnolis OS提供了二进制文件，请点击[这里](http://attestation.openanolis.cn/aas-client)下载.

# Attestation

在一个真实可信硬件的TEE中运行client，进行attestation测试：

```shell
aas-client --url http://attestation.openanolis.cn/1234567 attest
```
请注意将url参数替换为您的AAS实例URL.

这行命令将会在TEE搜集远程证明证据，并向AAS发起认证请求，在请求通过后，返回一个包含了证据验证结果且被签名的JWT令牌（Base64编码），而证据验证结果中包含了从证据中解析出的表征TEE可信度的重要内容，例如硬件安全版本号，软件度量值等等。

**注：**若暂时没有真实硬件上的TEE环境，可以设置如下环境变量后再运行aas-client，以使用Sample TEE类型来测试AAS的Attestation功能：

```shell
export AA_SAMPLE_ATTESTER_TEST=yes
```

当前支持的TEE平台类型包括：
- Intel TDX
- Intel SGX Occlum
- AMD SEV-SNP

## 获取令牌验证公钥
通过如下命令从AAS获取用于验证令牌签名的公钥：
```
curl -k -X GET http://attestation.openanolis.cn/1234567/kbs/v0/token-certificate-chain
```
这会返回一个JWKS格式的公钥，并包含自签名的x.509格式证书。

## 设置自定义Attestation策略

可以通过如下命令设置自定义的Attestation策略（rego语法），以在硬件厂商要求的基本的默认证据验证策略之外，附加上所需要的进一步的证据内容检查：
```shell
aas-client --url http://attestation.openanolis.cn/1234567 config --auth-private-key /etc/aas-auth.key set-attestation-policy --type rego --id default --policy-file /path/to/policy.rego
```

请注意将`/etc/aas-auth.key`替换为您的认证密钥，将`/path/to/policy.rego`替换为策略文件的路径.

若不设置自定义策略，则AAS仅会根据硬件厂商的默认策略检查TEE证据（未来龙蜥社区会考虑提供社区的安全基线策略）。

### 基础策略模版

在自定义策略文件中，可以对每一类TEE的TCB状态和度量值做额外的限制检查，例如要求TDX Quote中的`tcb_svn`和`mr_td`字段必须是几个给定的参考值中的一个，则可以设置策略如下：

my_policy.rego
```
package my_policy

import future.keywords.if

default allow = false

# The allowed reference values for the specific field in the quote
reference_tdx_tcb_svn = [ "03000500000000000000000000000000" ]
reference_tdx_mr_td = [ "abcd1234", "1234abcd", "a1b2c3d4" ]

allow if {
    input["tdx.quote.body.tcb_svn"] == reference_tdx_tcb_svn[_]
    input["tdx.quote.body.mr_td"] == reference_tdx_mrtd[_]
}
```

上述策略是对证据内容做出限制的基础模版，如果您不熟悉OPA策略引擎的rego语法，则可以直接修改上述模版添加更多的对证据内容字段的检查策略。
同时，AAS允许精通OPA策略rego语法的用户自行编写策略文件以实现任何想要的复杂策略逻辑。

AAS允许您在同一份策略文件中同时添加不同类型TEE的证据内容字段的检查策略。这些字段在解析出的序列化证据内容中有不同的名称，我们将每一类TEE所支持的证据内容字段名称列出如下：

#### Intel TDX

```
"tdx.quote.header.version"
"tdx.quote.header.att_key_type"
"tdx.quote.header.tee_type"
"tdx.quote.header.reserved"
"tdx.quote.header.vendor_id"
"tdx.quote.header.user_data"
"tdx.quote.body.mr_config_id"
"tdx.quote.body.mr_owner"
"tdx.quote.body.mr_owner_config"
"tdx.quote.body.mr_td"
"tdx.quote.body.mrsigner_seam"
"tdx.quote.body.report_data"
"tdx.quote.body.seam_attributes"
"tdx.quote.body.td_attributes"
"tdx.quote.body.mr_seam"
"tdx.quote.body.tcb_svn"
"tdx.quote.body.xfam"
```

#### Intel SGX Occlum

```
"sgx.mr-signer"
"sgx.mr-enclave"
```

#### AMD SEV-SNP

```
"snp.policy_abi_major"
"snp.policy_abi_minor"
"snp.policy_smt_allowed"
"snp.policy_migrate_ma"
"snp.policy_debug_allowed"
"snp.policy_single_socket"
"snp.policy_single_socket"
"snp.reported_tcb_bootloader"
"snp.reported_tcb_tee"
"snp.reported_tcb_snp"
"snp.reported_tcb_microcode"
"snp.platform_tsme_enabled"
"snp.platform_smt_enabled"
"snp.measurement"
```

# 机密数据托管

AAS基于TEE Attestation为社区用户提供了机密数据的上传和托管功能服务，以方便向运行时的TEE安全地注入机密数据。
TEE内的应用程序可使用AAS-Client向AAS实例发起数据获取请求，请求过程中强制执行AAS的TEE Attestation过程，完成对TEE真实性的验证，
并在验证通过后，向TEE返回加密的机密数据内容（解密密钥为TEE私钥）。

## 上传机密数据

在本地准备机密数据文件：

```shell
cat << EOF > key_data.txt
1234567890
EOF
```

为机密数据设置唯一的tag标识：AAS规定为机密数据设置的tag格式为`[top_tag]/[middle_tag]/[tag]`, 例如可以为一个解密密钥分配tag: `decryption_key/rsa_pub/key_1`

上传机密资源：

```shell
aas-client --url http://attestation.openanolis.cn/1234567 config --auth-private-key /etc/aas-auth.key set-resource --resource-file key_data.txt --path decryption_key/rsa_pub/key_1
```

上述命令样例中，我们将本地的`key_data.txt`文件中的机密数据上传到了AAS实例中，并为其分配了路径标识`decryption_key/rsa_pub/key_1`.

## 获取机密数据

机密数据的获取过程要求执行TEE Attestation，因此请求方需运行在真实可信的硬件TEE中。

AAS-Client Tool提供了获取机密数据的功能：

```shell
aas-client --url http://attestation.openanolis.cn/1234567 get-resource --path decryption_key/rsa_pub/key_1
```

上述命令会将获取到的Base64编码的机密数据明文内容打印出来。

