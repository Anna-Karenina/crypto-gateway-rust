//! # HTTP обработчики
//! 
//! Модули обработчиков HTTP запросов:
//! - `wallet` - операции с кошельками
//! - `transfer` - операции с переводами
//! - `debug` - отладочные endpoint'ы

pub mod debug;
pub mod token_handlers;
pub mod transfer;
pub mod wallet;

// Реэкспорт всех handlers для удобства
pub use debug::*;
pub use token_handlers::*;
pub use transfer::*;
pub use wallet::*;
