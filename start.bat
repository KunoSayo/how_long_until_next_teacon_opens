@echo off
REM Teacon Counter 快速启动脚本 (Windows)

echo ================================================
echo    Teacon Counter - 快速启动
echo ================================================
echo.

REM 检查 Docker 是否运行
docker info >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: Docker 未运行，请先启动 Docker Desktop
    pause
    exit /b 1
)

echo 正在构建并启动容器...
echo.

docker-compose up -d --build

echo.
echo ================================================
echo   应用已启动！
echo ================================================
echo.
echo 访问地址: http://localhost:8080
echo.
echo 常用命令:
echo   查看日志: docker logs -f teacon-counter
echo   停止应用: docker stop teacon-counter
echo   重启应用: docker restart teacon-counter
echo   删除容器: docker rm -f teacon-counter
echo.
docker ps --filter "name=teacon-counter"
echo.
pause
