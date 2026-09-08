# tokimo-app-rss

Tokimo 的常驻 RSS / Atom 订阅 App。它把 feed 的完整当前窗口保存到 PostgreSQL，提供历史搜索和关键词规则，并通过系统 `notification_center.notify` 提交通知。

## 语义

- App ID、数据库 schema、UDS service 均为 `rss`。
- 首次成功抓取只建立静默基线；只有之后新插入的条目参与规则匹配。新增或修改规则不会追溯发送。
- 规则使用 NFKC + 大小写归一化后的字面子串：分类满足、包含任一关键词、且不含任一排除词。
- 每个订阅源可以保存独立子视图；子视图基础条件在服务端与页面临时搜索条件叠加，并在过滤后进行游标分页。
- entry 与持久 delivery outbox 在同一事务创建；通知成功响应只表示系统已接受。失败按稳定 dedupe key 至少一次重试。
- 仅支持不含凭据的 HTTP(S) URL；初始 URL 和每次重定向都会重新检查协议与凭据。网络可达范围、DNS 结果及地址访问控制由部署环境的 dnsmasq 和网络策略负责。App 保留 15 秒总超时、5 MiB 响应上限和最多 3 次重定向的资源保护。
- 不读取 Cookie，不抓网页正文，不执行历史全站爬取，也不持有任何外部推送渠道凭据。

## API

宿主把 `/api/apps/rss/*` 认证后透明代理到 sidecar：

- `GET|POST /sources`, `PATCH /sources/{id}`, `POST /sources/test`, `POST /sources/{id}/refresh`
- `GET /entries`, `GET /entries/{id}`
- `GET|POST /views`, `PATCH|DELETE /views/{id}`
- `GET|POST /rules`, `PATCH|DELETE /rules/{id}`, `POST /rules/preview`
- `GET /deliveries`, `POST /notifications/test`

所有业务路由使用宿主注入的 `TokimoUser`，请求 body 不接收 `user_id`。

## 数据库与运行

宿主依据 `tokimo-app.toml` 创建 `rss` schema 并执行 `migrations/`；sidecar 自身不会建表。无子命令且存在 `TOKIMO_BUS_SOCKET` 时进入 resident server 模式，恢复采集调度和未完成 outbox；人工直接运行只打印帮助。

## 验证

```bash
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo metadata --locked
```

Rust DTO 通过 `ts-rs` 生成到 `ui/src/generated/rust-types/`。UI 构建由同仓 UI 工程负责。

## 安装

发布到可验证的远端 revision 后，再在 Tokimo 主仓 `feeds.toml` 注册 `apps/tokimo-app-rss`。本仓不会自行修改主仓 feed 配置。

License: MIT OR Apache-2.0.
