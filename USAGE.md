# 使用说明

## 功能概述

这是一个简单的倒计时应用，显示从预设时间（Unix 时间戳 0，即 1970-01-01 00:00:00 UTC）开始的周数和对应的日期时间。

## 核心逻辑

### 1. 首页访问（自动增加周数）
- **访问路径**: `GET /`
- **行为**: 每次访问首页时，会自动尝试增加周数
- **限制**: 每个 IP 地址在每个 UTC 日内只能成功增加一次
- **实现**: 使用 IP 检查机制防止重复计数

### 2. 按钮点击（手动增加周数）
- **访问路径**: `POST /api/increment`
- **行为**: 点击按钮时，永远会增加一周，无任何限制
- **目的**: 允许用户自由增加周数，不受 IP 限制

### 3. 数据查询
- **访问路径**: `GET /api/data`
- **行为**: 获取当前存储的周数

## 技术特点

### 异步数据库操作
所有数据库操作都使用 `tokio::task::spawn_blocking` 在后台线程池中执行：
- **优势**: 不阻塞 Actix Web 的工作线程，提高并发性能
- **实现**: Sled 数据库的 I/O 操作完全异步化

### IP 限流机制
- **存储方式**: 持久化存储在 Sled 数据库中
- **限流粒度**: IP 地址 + UTC 日期
- **有效期**: 每个 UTC 日（00:00:00 UTC 到次日 00:00:00 UTC）
- **支持的代理头**: X-Forwarded-For, X-Real-IP, CF-Connecting-IP

### 性能优化
- **首页响应**: 立即返回 HTML，后台异步处理增加逻辑
- **数据库刷新**: 异步进行，不等待完成
- **非阻塞设计**: 所有 I/O 操作都不阻塞 HTTP 请求处理

## API 接口

### GET /
首页，返回 HTML 页面

**行为**:
- 异步尝试增加周数（带 IP 检查）
- 立即返回页面内容

**示例**:
```bash
curl http://localhost:8080/
```

### GET /api/data
获取当前周数

**响应**:
```json
{
  "success": true,
  "week_count": 42
}
```

**示例**:
```bash
curl http://localhost:8080/api/data
```

### POST /api/increment
手动增加周数（无 IP 限制）

**响应**:
```json
{
  "success": true,
  "week_count": 43
}
```

**示例**:
```bash
curl -X POST http://localhost:8080/api/increment
```

### GET /health
健康检查

**响应**:
```json
{
  "status": "healthy",
  "service": "teacon-counter"
}
```

## 部署

### 使用 Docker Compose（推荐）
```bash
docker-compose up -d
```

### 使用启动脚本
```bash
# Linux/Mac
./start.sh

# Windows
start.bat
```

### 使用 Docker
```bash
docker build -t teacon-counter .
docker run -d \
  --name teacon-counter \
  -p 8080:8080 \
  -v teacon-data:/data \
  teacon-counter
```

## 环境变量

- `RUST_LOG`: 日志级别（默认: info）
- `BIND_ADDRESS`: 绑定地址（默认: 0.0.0.0:8080）
- `DB_PATH`: 数据库路径（默认: /data/db）

## 工作流程示例

### 场景 1: 首次访问
1. 用户访问 `http://localhost:8080/`
2. 系统检查该 IP 今天是否访问过
3. 如果未访问过，周数 +1
4. 记录该 IP 的访问时间
5. 返回页面显示新的周数

### 场景 2: 同一天内再次访问
1. 用户再次访问首页
2. 系统检测到该 IP 今天已经访问过
3. 周数不变
4. 返回页面显示当前周数

### 场景 3: 点击按钮
1. 用户点击"增加一周"按钮
2. 发送 POST 请求到 `/api/increment`
3. 系统直接增加周数（无 IP 检查）
4. 返回新的周数

## 注意事项

1. **数据持久化**: 所有数据都存储在 Sled 数据库中，容器重启后数据保留
2. **时间基准**: 使用 UTC 时区进行日期计算
3. **并发安全**: 使用数据库事务保证数据一致性
4. **性能**: 首页访问不会因为数据库操作而延迟响应

## 故障排除

### 查看日志
```bash
docker logs -f teacon-counter
```

### 检查数据库
```bash
docker exec -it teacon-counter ls -la /data
```

### 重置数据
```bash
docker exec -it teacon-counter rm -rf /data/*
docker restart teacon-counter
```

## 性能测试

使用 Apache Bench 进行压力测试：

```bash
# 测试首页访问
ab -n 10000 -c 100 http://localhost:8080/

# 测试 API 接口
ab -n 10000 -c 100 -p /dev/null -T application/json http://localhost:8080/api/increment
```

## 开发

### 本地运行
```bash
cargo run
```

### 运行测试
```bash
cargo test
```

### 代码检查
```bash
cargo clippy
```

## 许可证

MIT License
