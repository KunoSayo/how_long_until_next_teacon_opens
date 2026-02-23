# 更新日志

## 最新更新 - 异步数据库操作

### 主要改进

#### 1. 异步数据库操作
- 所有数据库操作现在使用 `tokio::task::spawn_blocking` 在后台线程池中执行
- 不会阻塞 Actix Web 的工作线程，提高并发性能
- 数据库操作与 HTTP 请求处理完全异步化

#### 2. 改进的错误处理
- 引入自定义 `DbError` 错误类型
- 实现 `Send` trait，支持跨线程传递
- 使用 `thiserror` 简化错误处理代码

#### 3. 首页自动增加周数
- 访问首页 `/` 时会自动尝试增加周数
- 使用 `tokio::spawn` 异步处理，不阻塞页面响应
- 保持原有的 IP 限流机制（每个 UTC 日内只能点击一次）

### 技术细节

#### 异步操作流程
```
HTTP 请求 → Actix 工作线程 → 异步数据库操作 → 后台线程池 → Sled 数据库
                ↓
            立即返回/继续处理其他请求
```

#### 性能优势
- **非阻塞**: 数据库 I/O 操作不会阻塞工作线程
- **高并发**: 可以同时处理更多 HTTP 请求
- **后台刷新**: 数据持久化异步进行，不等待完成

### API 变更

#### 数据库方法 (全部异步化)
```rust
// 获取当前周数
pub async fn get_week_count(&self) -> Result<u64, DbError>

// 增加周数
pub async fn increment_week(&self, ip: String) -> Result<bool, DbError>

// 获取完整数据
pub async fn get_week_data(&self) -> Result<WeekData, DbError>

// 重置周数
pub async fn reset_weeks(&self) -> Result<(), DbError>
```

#### 路由变更
- `GET /` - 首页，访问时自动尝试增加周数（异步）
- `GET /api/data` - 获取当前周数
- `POST /api/increment` - 手动增加周数
- `GET /health` - 健康检查

### 依赖更新

```toml
[dependencies]
# 新增依赖
tokio = { version = "1.40", features = ["full"] }
thiserror = "2.0"
```

### 使用示例

#### 启动服务
```bash
docker-compose up -d
```

#### 访问首页（自动增加周数）
```bash
curl http://localhost:8080/
```

#### 查看当前周数
```bash
curl http://localhost:8080/api/data
```

### 注意事项

1. **IP 限流**: 无论访问首页还是调用 API，每个 IP 在每个 UTC 日内只能成功增加一次周数
2. **异步处理**: 首页访问的增加操作是异步的，页面会立即返回，后台处理增加逻辑
3. **数据一致性**: 使用 Sled 的 ACID 事务保证数据一致性

### 部署

无需更改部署流程，现有的 Docker 配置完全兼容：

```bash
# 使用 Docker Compose
docker-compose up -d

# 或使用启动脚本
./start.sh  # Linux/Mac
start.bat   # Windows
```

### 性能测试建议

1. 使用 Apache Bench 或 wrk 进行压力测试
2. 观察日志中的异步操作执行情况
3. 对比同步和异步版本的吞吐量差异

```bash
# 压力测试示例
ab -n 10000 -c 100 http://localhost:8080/
```
