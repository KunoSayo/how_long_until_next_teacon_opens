# 部署指南

## 前置要求

- Docker (推荐 20.10+)
- Docker Compose (可选)

## 快速部署

### 方法 1: 使用启动脚本

**Linux/Mac:**
```bash
chmod +x start.sh
./start.sh
```

**Windows:**
```cmd
start.bat
```

### 方法 2: 使用 Docker Compose

```bash
# 构建并启动
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止
docker-compose down
```

### 方法 3: 使用 Docker

```bash
# 构建镜像
docker build -t teacon-counter .

# 运行容器
docker run -d \
  --name teacon-counter \
  -p 8080:8080 \
  -v teacon-data:/data \
  --restart unless-stopped \
  teacon-counter
```

## 访问应用

部署完成后，访问 http://localhost:8080

如果部署在服务器上，将 localhost 替换为服务器的 IP 地址或域名。

## 环境变量配置

可以通过环境变量自定义配置：

```bash
docker run -d \
  --name teacon-counter \
  -p 8080:8080 \
  -v teacon-data:/data \
  -e RUST_LOG=debug \
  -e BIND_ADDRESS=0.0.0.0:9000 \
  teacon-counter
```

可用的环境变量：
- `RUST_LOG`: 日志级别 (trace, debug, info, warn, error)
- `BIND_ADDRESS`: 绑定地址 (默认: 0.0.0.0:8080)
- `DB_PATH`: 数据库路径 (默认: /data/db)

## 数据备份

备份数据卷：
```bash
docker run --rm \
  -v teacon-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/teacon-data-backup.tar.gz /data
```

恢复数据：
```bash
docker run --rm \
  -v teacon-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/teacon-data-backup.tar.gz -C /
```

## 查看日志

```bash
# 实时日志
docker logs -f teacon-counter

# 最近 100 行
docker logs --tail 100 teacon-counter
```

## 故障排除

### 容器无法启动

```bash
# 检查容器状态
docker ps -a

# 查看容器日志
docker logs teacon-counter
```

### 端口冲突

如果 8080 端口被占用，可以使用其他端口：

```bash
docker run -d \
  --name teacon-counter \
  -p 9000:8080 \
  teacon-counter
```

### 数据持久化问题

确保正确挂载了数据卷：

```bash
docker inspect teacon-counter | grep -A 10 Mounts
```

## 生产环境部署建议

1. **使用反向代理** (如 Nginx)
2. **配置 HTTPS** (使用 Let's Encrypt)
3. **设置防火墙规则**
4. **定期备份数据**
5. **监控容器状态**

### Nginx 配置示例

```nginx
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl;
    server_name your-domain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## 性能优化

- 调整 Docker 资源限制
- 使用多阶段构建减小镜像大小
- 配置适当的日志级别
