use actix_web::web;

use super::handlers::*;

/// Конфигурация всех HTTP маршрутов
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Простой корневой маршрут для проверки
    cfg.route("/", web::get().to(root_handler));

    cfg.service(
        web::scope("/api")
            .service(
                // Маршруты для кошельков
                web::scope("/wallets")
                    .route("", web::post().to(create_wallet))
                    .route("/{wallet_id}", web::get().to(get_wallet))
                    .route("/{wallet_id}/balance", web::get().to(get_wallet_balance))
                    .route(
                        "/{wallet_id}/transactions",
                        web::get().to(get_wallet_transactions),
                    )
                    .route(
                        "/activate/{wallet_address}",
                        web::post().to(activate_wallet),
                    ),
            )
            .service(
                // Маршруты для трансферов
                web::scope("/transfers")
                    .route("/preview", web::post().to(preview_transfer))
                    .route("", web::post().to(create_transfer))
                    .route("/{transfer_id}", web::get().to(get_transfer))
                    .route(
                        "/by-reference/{reference_id}",
                        web::get().to(get_transfer_by_reference),
                    )
                    .route("/wallet/{wallet_id}", web::get().to(get_wallet_transfers))
                    .route(
                        "/process-pending",
                        web::post().to(process_pending_transfers),
                    ),
            )
            .service(
                // Маршруты для транзакций
                web::scope("/transactions").route("/{tx_hash}", web::get().to(get_transaction)),
            )
            .service(
                // 🪙 Мультитокенные маршруты (новые!)
                web::scope("/tokens")
                    .route("", web::get().to(get_supported_tokens))
                    .route("/balance", web::get().to(get_multi_token_balance))
                    .route("/transfer", web::post().to(create_multi_token_transfer))
                    .route("/{token_symbol}/toggle", web::post().to(toggle_token_status))
                    .route("/cache/stats", web::get().to(get_cache_stats_and_cleanup))
                    .route("/cache/invalidate/{wallet_address}", web::delete().to(invalidate_wallet_cache)),
            )
            .service(
                // Отладочные маршруты
                web::scope("/debug")
                    .route(
                        "/master-wallet/balance",
                        web::get().to(get_master_wallet_balance),
                    )
                    .route("/system/health", web::get().to(health_check)),
            ),
    );
}
