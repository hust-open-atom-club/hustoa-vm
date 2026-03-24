# CHANGELOG

## [0.1.2] - 2025-10-20

- 修改了部分下载链接（ustc 不再支持通过 wget 下载）
- 修改 config.toml 中 `libvirt_interface` 配置为 `libvirt_network`
- 支持自动配置 NAT 端口转发（需要 `hustoa-vm install-hook` ）
- 支持手动更新软件（`hustoa-vm self-update`）

## [0.1.1] - 2024-09-17

- 增加了 `hustoa-vm v6-pool list` 命令
- 自动将旧的 `v6pool.list` 数据迁移到 `v6pool.toml` 中，但需要手动修改 domain 字段以避免被 `purge` 命令清理
- 支持了 archlinux 发行版（不稳定的支持，不保证功能正常）

## [0.1.0] - 2024-09-16

实现了基本功能。

[0.1.0]: https://github.com/hust-open-atom-club/hustoa-vm/releases/tag/v0.1.0
[0.1.1]: https://github.com/hust-open-atom-club/hustoa-vm/releases/tag/v0.1.1
[0.1.2]: https://github.com/hust-open-atom-club/hustoa-vm/releases/tag/v0.1.2
