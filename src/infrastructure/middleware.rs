//! # Middleware для HTTP API
//!
//! Rate limiting, audit logging и другие middleware

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, Result,
};
use futures_util::future::{ok, Ready};
use serde_json::json;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::{info, warn};

/// Rate limiter на основе IP адресов
#[derive(Clone)]
pub struct RateLimiter {
    max_requests: u32,
    window_duration: Duration,
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn check_rate_limit(&self, ip: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        let ip_requests = requests.entry(ip.to_string()).or_insert_with(Vec::new);

        // Удаляем старые запросы
        ip_requests.retain(|&time| now.duration_since(time) < self.window_duration);

        if ip_requests.len() >= self.max_requests as usize {
            warn!("🚫 Rate limit превышен для IP: {}", ip);
            false
        } else {
            ip_requests.push(now);
            true
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RateLimiterMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimiterMiddleware {
            service: Rc::new(service),
            rate_limiter: self.clone(),
        })
    }
}

pub struct RateLimiterMiddleware<S> {
    service: Rc<S>,
    rate_limiter: RateLimiter,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =
        futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let rate_limiter = self.rate_limiter.clone();

        Box::pin(async move {
            // Получаем IP адрес
            let ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            if !rate_limiter.check_rate_limit(&ip) {
                return Err(actix_web::error::ErrorTooManyRequests(
                    json!({
                        "error": "Rate limit exceeded",
                        "message": "Слишком много запросов, попробуйте позже"
                    })
                    .to_string(),
                ));
            }

            service.call(req).await
        })
    }
}

/// Audit logger middleware
pub struct AuditLogger;

impl<S, B> Transform<S, ServiceRequest> for AuditLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuditLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuditLoggerMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct AuditLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuditLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future =
        futures_util::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let start_time = Instant::now();

        // Логируем информацию о запросе
        let method = req.method().clone();
        let path = req.path().to_string();
        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        Box::pin(async move {
            let response = service.call(req).await;

            let duration = start_time.elapsed();

            match &response {
                Ok(res) => {
                    let status = res.status().as_u16();

                    if status >= 400 {
                        warn!(
                            "📝 API Request: {} {} - {} - {} - {:?} - {}",
                            method, path, status, ip, duration, user_agent
                        );
                    } else {
                        info!(
                            "📝 API Request: {} {} - {} - {} - {:?}",
                            method, path, status, ip, duration
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        "📝 API Request ERROR: {} {} - {} - {:?} - {}",
                        method, path, ip, duration, err
                    );
                }
            }

            response
        })
    }
}

/// Конфигурация middleware
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    pub rate_limit_enabled: bool,
    pub rate_limit_requests_per_minute: u32,
    pub audit_logging_enabled: bool,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            rate_limit_enabled: true,
            rate_limit_requests_per_minute: 60, // 60 запросов в минуту
            audit_logging_enabled: true,
        }
    }
}
