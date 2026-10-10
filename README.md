# tokimo-app-rss

Tokimo 的常驻 RSS / Atom 订阅 App。它把 feed 的完整当前窗口保存到 PostgreSQL，提供历史搜索和关键词规则，并通过系统 `notification_center.notify` 提交通知。

## 语义

- App ID、数据库 schema、UDS service 均为 `rss`。
- 首次成功抓取只建立静默基线；只有之后新插入的条目参与规则匹配。新增或修改规则不会追溯发送。
- 规则使用 NFKC + 大小写归一化后的字面子串：分类满足、包含任一关键词、且不含任一排除词。
- 每个订阅源可以保存独立子视图；子视图基础条件在服务端与页面临时搜索条件叠加，并在过滤后进行游标分页。
- entry 与持久 delivery outbox 在同一事务创建；通知成功响应只表示系统已接受。失败按稳定 dedupe key 至少一次重试。
- 仅支持不含凭据的 HTTP(S) URL；初始 URL 和每次重定向都会重新检查协议与凭据。网络可达范围、DNS 结果及地址访问控制由部署环境的 dnsmasq 和网络策略负责。App 保留单次目标 HTTP 请求 15 秒超时、5 MiB 响应上限和最多 3 次重定向的资源保护；未配置解挑战服务时抓取总期限为 15 秒，配置后为 90 秒。
- 不读取用户 Cookie，不抓网页正文，不执行历史全站爬取，也不持有任何外部推送渠道凭据。

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

### Cloudflare 抓取

RSS 通过 `tokimo-web-fetch` 的原始响应入口抓取。普通 feed 直接使用 HTTP；遇到 `cf-mitigated: challenge` 时，调用 `FLARESOLVERR_URL` 指定的 FlareSolverr 服务获取站点范围内的 Cookie 和浏览器 User-Agent，再重发原始请求。RSS 解析真实 HTTP 返回的 XML，保留 ETag、Last-Modified、304、Retry-After、逐跳重定向校验和 5 MiB 流式上限；不会把浏览器渲染的 HTML 当作 feed。

在宿主环境配置 `FLARESOLVERR_URL`（例如 `http://flaresolverr:8191`），由 sidecar 继承。服务需要单独部署，且与 RSS 使用相同公网出口；当前接入不会自动创建服务。未配置时普通订阅仍正常，受挑战的源会记录明确的配置缺失错误。解挑战单次最多 60 秒，调用服务的 HTTP 请求额外保留 5 秒收尾时间；配置服务后每次 feed 抓取总期限为 90 秒，目标 HTTP 请求仍分别限制为 15 秒、连接为 8 秒。定时和手动采集统一使用 120 秒租约，为解挑战和持久化留出时间，避免解挑战期间被再次采集；浏览器启动、挑战或传输超时会按现有失败退避重试。Cookie 只用于当前抓取，不落库、不跨订阅源共享；复用 Cookie 也不能保证通过依赖浏览器传输指纹的防护。FlareSolverr 浏览器自身的重定向和网络访问由部署网络策略限制。

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
