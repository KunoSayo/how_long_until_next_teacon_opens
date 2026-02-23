#!/bin/bash

# Teacon Counter 快速启动脚本

set -e

echo "🚀 Teacon Counter - 快速启动"
echo "================================"

# 检查 Docker 是否安装
if ! command -v docker &> /dev/null; then
    echo "❌ 错误: 未找到 Docker，请先安装 Docker"
    exit 1
fi

# 检查 Docker Compose 是否可用
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ 错误: 未找到 Docker Compose，请先安装"
    exit 1
fi

echo "📦 构建并启动容器..."

# 使用 docker compose 或 docker-compose
if docker compose version &> /dev/null; then
    docker compose up -d --build
else
    docker-compose up -d --build
fi

echo ""
echo "✅ 应用已启动！"
echo ""
echo "📝 访问地址: http://localhost:8080"
echo ""
echo "🔧 常用命令:"
echo "  查看日志: docker logs -f teacon-counter"
echo "  停止应用: docker stop teacon-counter"
echo "  重启应用: docker restart teacon-counter"
echo "  删除容器: docker rm -f teacon-counter"
echo ""
echo "📊 查看状态:"
docker ps --filter "name=teacon-counter"
