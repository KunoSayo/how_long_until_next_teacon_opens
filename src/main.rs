mod db;

use actix_web::{web, App, HttpResponse, HttpServer, Responder, HttpRequest};
use actix_cors::Cors;
use serde::Serialize;
use std::sync::Arc;
use db::Database;

/// API 响应结构
#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    week_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// 获取客户端 IP 地址
fn get_client_ip(req: &HttpRequest, connection_info: &actix_web::dev::ConnectionInfo) -> String {
    // 尝试从 X-Forwarded-For 头获取真实 IP
    if let Some(forwarded_for) = req.headers().get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // X-Forwarded-For 可能包含多个 IP，取第一个
            let first_ip = forwarded_str.split(',').next().unwrap_or("");
            let trimmed = first_ip.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    // 尝试从 X-Real-IP 头获取
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // 尝试从 CF-Connecting-IP (Cloudflare) 头获取
    if let Some(cf_ip) = req.headers().get("CF-Connecting-IP") {
        if let Ok(ip_str) = cf_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // 回退到远程地址
    connection_info
        .realip_remote_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 首页路由 - 访问时自动增加一周（带 IP 检查）
async fn index(
    db: web::Data<Arc<Database>>,
    req: HttpRequest,
    connection_info: actix_web::dev::ConnectionInfo,
) -> impl Responder {
    let client_ip = get_client_ip(&req, &connection_info);
    log::info!("首页访问，来自 IP: {}", client_ip);

    // 尝试增加周数（带 IP 检查，异步处理不阻塞响应）
    let db_clone = db.clone();
    let client_ip_clone = client_ip.clone();
    tokio::spawn(async move {
        match db_clone.increment_week_with_ip_check(client_ip_clone).await {
            Ok(true) => {
                if let Ok(week_count) = db_clone.get_week_count().await {
                    log::info!("访问首页成功增加周数，当前周数: {}", week_count);
                }
            }
            Ok(false) => {
                log::info!("IP {} 在当前时间窗口内已经访问过首页", client_ip);
            }
            Err(e) => {
                log::error!("访问首页时增加周数失败: {}", e);
            }
        }
    });

    // 返回首页内容
    let html = include_str!("index.html");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

/// 获取当前数据 API（带 IP 检查，如果当天没有记录则自动增加一周）
async fn get_data(
    db: web::Data<Arc<Database>>,
    req: HttpRequest,
    connection_info: actix_web::dev::ConnectionInfo,
) -> impl Responder {
    let client_ip = get_client_ip(&req, &connection_info);
    log::info!("获取数据请求，来自 IP: {}", client_ip);

    // 尝试增加周数（带 IP 检查）
    match db.increment_week_with_ip_check(client_ip.clone()).await {
        Ok(_) => {
            // 无论是否增加，都返回当前周数
            match db.get_week_count().await {
                Ok(week_count) => {
                    log::info!("返回当前周数: {}", week_count);
                    HttpResponse::Ok().json(ApiResponse {
                        success: true,
                        week_count,
                        message: None,
                    })
                }
                Err(e) => {
                    log::error!("获取数据失败: {}", e);
                    HttpResponse::InternalServerError().json(ApiResponse {
                        success: false,
                        week_count: 0,
                        message: Some("获取数据失败".to_string()),
                    })
                }
            }
        }
        Err(e) => {
            log::error!("增加周数失败: {}", e);
            // 即使增加失败，也尝试返回当前周数
            match db.get_week_count().await {
                Ok(week_count) => HttpResponse::Ok().json(ApiResponse {
                    success: true,
                    week_count,
                    message: None,
                }),
                Err(_) => HttpResponse::InternalServerError().json(ApiResponse {
                    success: false,
                    week_count: 0,
                    message: Some("操作失败".to_string()),
                }),
            }
        }
    }
}

/// 增加周数 API（无 IP 检查，永远增加）
async fn increment_week(
    db: web::Data<Arc<Database>>,
) -> impl Responder {
    log::info!("收到增加周数请求（按钮点击）");

    match db.increment_week().await {
        Ok(week_count) => {
            log::info!("成功增加周数，当前周数: {}", week_count);
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                week_count,
                message: None,
            })
        }
        Err(e) => {
            log::error!("增加周数失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                week_count: 0,
                message: Some("操作失败，请稍后重试".to_string()),
            })
        }
    }
}

/// 健康检查 API
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "teacon-counter"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // 数据库路径
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "./data/db".to_string());

    // 初始化数据库
    let db = match Database::new(&db_path) {
        Ok(database) => {
            log::info!("数据库初始化成功，路径: {}", db_path);
            Arc::new(database)
        }
        Err(e) => {
            log::error!("数据库初始化失败: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("数据库初始化失败: {}", e),
            ));
        }
    };

    // 服务器地址
    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    log::info!("启动服务器，监听地址: {}", bind_address);

    // 启动 HTTP 服务器
    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::new(db.clone()))
            .wrap(cors)
            .route("/", web::get().to(index))
            .route("/api/data", web::get().to(get_data))
            .route("/api/increment", web::post().to(increment_week))
            .route("/health", web::get().to(health_check))
    })
    .bind(&bind_address)?
    .run()
    .await
}
