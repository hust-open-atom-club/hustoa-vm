# hustoa-vm

一个简单的 libvirt 虚拟机维护工具。

## 基本功能

- 创建虚拟机：以最小化的配置创建虚拟机
- 管理 ipv6 网络：为虚拟机提供校园网内可访问的 ipv6 地址
- 一键保存和恢复虚拟机状态

设计原则：

- 该工具应与 libvirt 工具（如 virsh、cockpit-machines）配合使用，而不是作为 libvirt 工具的封装
- 以 root 用户运行而设计，不保证非 root 用户下运行的行为正确
- 仅为华中科技大学校园内网设计

## 构建

```sh
apt install libssl-dev pkg-config cloud-image-utils
cargo build --release
```

如需安装，可使用以下命令

```sh
cargo install --path . --root /usr/local

# or just copy the file
cp ./target/release/hustoa-vm /usr/local/bin/hustoa-vm
```

## 使用方法

### 设置网络环境

如果需要使用 ipv6，需要在 libvirt 中添加一个仅 v6 的网络。该网络的 xml 配置可用以下命令生成：

```bash
hustoa-vm v6-pool gen-v6-net-xml > netdefine.xml
```

然后使用 `virsh` 工具定义该网络：

```bash
virsh net-define --file netdefine.xml
```

此外，需要开启系统 ipv6 的 proxy ndp，可在 `/etc/sysctl.conf` 中添加：

```
net.ipv6.conf.all.proxy_ndp = 1
```

为使配置生效，可重启系统或执行 `sysctl -p` 命令

### 配置文件

配置文件路径为 `/etc/hustoa-vm/config.toml`，该文件示例如下：

```toml
[common]
# 虚拟机镜像的存储路径，默认值为 /var/lib/libvirt/images
libvirt_storage = "/var/lib/libvirt/images"
# libvirt 默认保存虚拟机状态的文件夹，默认值为 /var/lib/libvirt/qemu/save
libvirt_save = "/var/lib/libvirt/qemu/save"
# 创建虚拟机使用的默认网络接口，默认为 virbr0
libvirt_interface = "virbr0"

# hustoa-vm create 默认使用的磁盘镜像大小，单位为 GiB
default_disk_size = 80
# hustoa-vm create 默认使用的内存大小，单位为 GiB
default_memory_size = 16
# hustoa-vm create 默认使用的 cpu 数量
default_vcpus = 16

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

详情可参见 [src/config.rs](src/config.rs)

### 创建虚拟机

**最小化配置：**

下面的命令将创建一个 latest ubuntu server 虚拟机，并具有 60G 磁盘空间、16G 内存和 16 个 vcpu

```sh
hustoa-vm create \
  --name sophie \
  --ssh-pubkey ~/.ssh/id_ed25519.pub \
  --distro ubuntu
```

**目前支持的完整配置：**

```bash
hustoa-vm create \
  --name sophie \
  --user sophie \
  --ssh-pubkey ~/.ssh/id_ed25519.pub \
  --distro ubuntu
  --distro-version jammy \
  --disk-size 60 \
  --memory 16 \
  --vcpus 16
```

按需调整配置即可。如果提供了 `--user` 选项，创建虚拟机时使用的默认用户名则来源于该选项，否则将 fallback 到 `--name` 的值。

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
- Debian Bookworm amd64
- OpenEuler 22.03 LTS aarch64

## TODO

- [ ] 支持其他发行版的镜像下载与基本配置
- [x] 批量暂停虚拟机并保存状态
- [x] 根据 libvirt 的配置删除不需要的 v6 地址
- [x] 配置文件中设置默认的磁盘、内存等大小
