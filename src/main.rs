use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use tron_gateway_rust::{
    infrastructure::{grpc::GrpcServer, http::configure_routes},
    AppState, Settings, VERSION,
};

#[actix_web::main]
async fn main() -> Result<()> {
    // Инициализация логирования
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Загрузка конфигурации из config.toml и переменных окружения
    let settings =
        Settings::new().map_err(|e| anyhow::anyhow!("Ошибка загрузки конфигурации: {}", e))?;
    info!("🚀 TRON Gateway v{} запускается...", VERSION);
    info!("Конфигурация загружена из config.toml и переменных окружения");
    info!(
        "HTTP сервер: {}:{}",
        settings.server.host, settings.server.port
    );
    info!(
        "gRPC сервер: {} ({}:{})",
        if settings.grpc.enabled {
            "включен"
        } else {
            "отключен"
        },
        settings.grpc.host,
        settings.grpc.port
    );

    // Инициализация состояния приложения (БД, сервисы)
    let app_state = Arc::new(AppState::new(settings.clone()).await?);
    info!("Сервисы инициализированы");

    // Запуск обоих серверов параллельно
    let http_bind = format!("{}:{}", settings.server.host, settings.server.port);
    let grpc_bind = format!("{}:{}", settings.grpc.host, settings.grpc.port);

    info!("🌐 HTTP сервер запускается на {}", http_bind);
    if settings.grpc.enabled {
        info!("⚡ gRPC сервер запускается на {}", grpc_bind);
    }

    // Создаем HTTP сервер
    let app_state_http = app_state.clone();
    let http_server = async move {
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new((*app_state_http).clone()))
                .wrap(Logger::default())
                .configure(configure_routes)
        })
        .bind(&http_bind)?
        .run()
        .await
        .map_err(anyhow::Error::from)
    };

    // Создаем gRPC сервер
    let grpc_server = async {
        if settings.grpc.enabled {
            let grpc_server = GrpcServer::new(settings.grpc.clone(), app_state.clone());
            grpc_server.serve().await?;
        }
        Ok::<(), anyhow::Error>(())
    };

    // Запускаем оба сервера параллельно
    tokio::try_join!(http_server, grpc_server)?;

    Ok(())
}
