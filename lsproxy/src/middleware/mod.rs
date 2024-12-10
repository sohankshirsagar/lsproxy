use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use std::env;

pub fn is_auth_enabled() -> bool {
    env::var("USE_AUTH").map(|v| v == "true").unwrap_or(false)
}
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
}

pub struct JwtMiddleware;

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareService { service }))
    }
}

pub struct JwtMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers().get("Authorization");
        
        if let Some(auth_header) = auth_header {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ");
                    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string());
                    
                    match decode::<Claims>(
                        token,
                        &DecodingKey::from_secret(secret.as_bytes()),
                        &Validation::default(),
                    ) {
                        Ok(_) => {
                            let fut = self.service.call(req);
                            return Box::pin(async move {
                                let res = fut.await?;
                                Ok(res)
                            });
                        }
                        Err(_) => {
                            return Box::pin(async move {
                                Err(actix_web::error::ErrorUnauthorized("Invalid token"))
                            });
                        }
                    }
                }
            }
        }
        
        Box::pin(async move {
            Err(actix_web::error::ErrorUnauthorized("Missing or invalid authorization header"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::{self, TestRequest};
    use actix_web::{web, App, HttpResponse};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn test_valid_token() {
        std::env::set_var("JWT_SECRET", "test_secret");
        
        let claims = Claims {
            exp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize + 3600,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("test_secret".as_bytes()),
        )
        .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(JwtMiddleware)
                .route("/", web::get().to(test_handler)),
        )
        .await;

        let req = TestRequest::get()
            .uri("/")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_invalid_token() {
        std::env::set_var("JWT_SECRET", "test_secret");
        
        let app = test::init_service(
            App::new()
                .wrap(JwtMiddleware)
                .route("/", web::get().to(test_handler)),
        )
        .await;

        let req = TestRequest::get()
            .uri("/")
            .insert_header(("Authorization", "Bearer invalid_token"))
            .to_request();

        let err = test::try_call_service(&app, req).await.unwrap_err();
        let resp = err.error_response();
        assert_eq!(resp.status().as_u16(), 401);
    }

    #[actix_web::test]
    async fn test_missing_auth_header() {
        let app = test::init_service(
            App::new()
                .wrap(JwtMiddleware)
                .route("/", web::get().to(test_handler)),
        )
        .await;

        let req = TestRequest::get().uri("/").to_request();
        let err = test::try_call_service(&app, req).await.unwrap_err();
        let resp = err.error_response();
        assert_eq!(resp.status().as_u16(), 401);
    }
}
