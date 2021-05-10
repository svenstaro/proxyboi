use actix_web::{
    client::{ClientResponse, SendRequestError},
    error::PayloadError,
    http::StatusCode,
    HttpResponse, ResponseError,
};
use log::error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyboiError {
    #[error("Unknown Internal Error")]
    SendRequestError(#[from] SendRequestError),
    #[error("Unknown Internal Error")]
    PayloadError(#[from] PayloadError),
    #[error("Unknown Internal Error")]
    Unknown(#[from] anyhow::Error),
}

impl ResponseError for ProxyboiError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self);
        HttpResponse::InternalServerError().finish()
    }
}
