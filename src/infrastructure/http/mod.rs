//! # HTTP инфраструктура
//!
//! HTTP сервер и маршрутизация:
//! - `handlers` - обработчики HTTP запросов
//! - `routes` - конфигурация маршрутов

pub mod handlers;
pub mod routes;

// Реэкспорт для удобства
pub use routes::configure_routes;
