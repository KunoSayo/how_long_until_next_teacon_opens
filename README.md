# Teacon 开放倒计时

一个使用 Rust 和 Docker 构建的简单倒计时应用。

## 功能特性

- 📊 显示从预设时间（2024-01-01 00:00:00 UTC）开始的周数
- 📅 自动计算并显示目标日期时间（处理溢出循环）
- 🔘 点击按钮增加周数
- 🛡️ IP 级别的防重复点击保护（每个 UTC 日内每个 IP 只能点击一次）
- 💾 使用 Sled 数据库进行高性能持久化存储
- 🐳 完整的 Docker 部署支持
- 🔄 健康检查和自动重启

## 技术栈

- **后端**: Rust + Actix Web
- **数据库**: Sled (嵌入式键值存储)
- **前端**: 原生 HTML/CSS/JavaScript
- **容器化**: Docker + Docker Compose

## 快速开始

### 使用 Docker Compose（推荐）

1. 克隆或下载项目
2. 在项目根目录运行：

```bash
docker-compose up -d
```

3. 访问 http://localhost:8080

### 使用 Docker

1. 构建镜像：

```bash
docker build -t teacon-counter .
```

2. 运行容器：

```bash
docker run -d \
  --name teacon-counter \
  -p 8080:8080 \
  -v teacon-data:/data \
  teacon-counter
```

### 本地开发

1. 安装 Rust 工具链

2. 运行开发服务器：

```bash
cargo run
```

3. 访问 http://localhost:8080

## 环境变量

- `DB_PATH`: 数据库存储路径（默认：`./data/db`）
- `BIND_ADDRESS`: 绑定地址（默认：`0.0.0.0:8080`）
- `RUST_LOG`: 日志级别（默认：`info`）

## API 接口

### GET /
返回主页面

### GET /api/data
获取当前周数
```json
{
  "success": true,
  "week_count": 42
}
```

### POST /api/increment
增加周数
```json
{
  "success": true,
  "week_count": 43
}
```

失败时：
```json
{
  "success": false,
  "week_count": 42,
  "message": "您在当前时间窗口内已经点击过了，请稍后再试"
}
```

### GET /health
健康检查
```json
{
  "status": "healthy",
  "service": "teacon-counter"
}
```

## 数据持久化

应用使用 Docker volume 持久化数据，即使容器重启或删除，数据也会保留。

默认数据存储在容器的 `/data` 目录，可以通过以下命令查看：

```bash
docker exec -it teacon-counter ls -la /data
```

## 防重复点击机制

- 每个 IP 地址在每个 UTC 日（00:00:00 UTC 到次日 00:00:00 UTC）内只能点击一次
- 基于 IP 地址进行限制，支持代理环境下的真实 IP 获取
- 点击记录存储在数据库中

## 项目结构

```
.
├── Cargo.toml              # Rust 项目配置
├── Cargo.lock              # 依赖锁定文件
├── Dockerfile              # Docker 构建配置
├── docker-compose.yml      # Docker Compose 配置
├── .dockerignore          # Docker 忽略文件
├── .gitignore             # Git 忽略文件
├── src/
│   ├── main.rs            # 主程序入口和 HTTP 服务器
│   ├── db.rs              # 数据库操作模块
│   └── index.html         # 前端页面
└── README.md              # 项目文档
```

## 许可证

MIT License
