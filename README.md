# hustoa-vm

一个简单的 libvirt 虚拟机维护工具。

## 基本功能

- 创建虚拟机：以最小化的配置创建虚拟机
- 管理 ipv6 网络：为虚拟机提供校园网内可访问的 ipv6 地址

设计原则：

- 该工具可与 libvirt 工具（如 virsh、cockpit-machines）配合使用
- 仅为华中科技大学校园内网设计

## 构建

```sh
apt install libssl-dev pkg-config cloud-image-utils
cargo build --release
```

如需安装，可使用以下命令

```sh
cargo install --path . --root /usr/local
```

## 使用方法

### 配置文件

配置文件路径为 `/etc/hustoa-vm/config.toml`，该文件示例如下：

```toml
[common]
# 虚拟机镜像的存储路径，默认值为 /var/lib/libvirt/images
libvirt_storage = "/var/lib/libvirt/images"
# 创建虚拟机使用的默认网络接口，默认为 virbr0
libvirt_interface = "virbr0"

# ipv6 相关配置，若不使用 ipv6 网络，该字段可省略
[ipv6conf]
# ipv6 的网络接口
libvirt_interface_v6 = "virbr1"
# 上述网络接口的 mac 地址
ipv6_bridge_mac = "52:54:00:48:8b:aa"
# host 所在网段的 ipv6 地址前缀
ipv6_prefix = "2001:250:4000:511d::"
# host 出口网络 interface
wan_interface = "eth0"
```

### 创建虚拟机

```sh
hustoa-vm create \
  --name sophie \
  --ssh-pubkey ~/.ssh/id_ed25519.pub \
  --distro ubuntu \
  --distro-version jammy \
  --disk-size 60 \
  --memory 16 \
  --vcpus 16
```

按需调整配置即可

### 持久化 ipv6 配置

可以通过在网络启动时执行以下命令进行配置

```sh
#!/bin/sh
hustoa-vm v6-pool flush
```

对于使用 NetworkManager 管理网络的宿主机，可以将以上内容保存在 `/etc/NetworkManager/dispatcher.d` 中

## 开发/测试状态

已测试的环境：

- Ubuntu 24.04 amd64
- Debian Bookworm

## TODO

- [ ] 批量暂停虚拟机并保存状态
- [ ] 支持其他发行版的镜像下载与基本配置
- [ ] 根据 libvirt 的配置删除不需要的 v6 地址
