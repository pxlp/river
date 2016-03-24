
use std::fmt::Debug;
use pon::*;

pub trait OutMessage: ToPon + Send + Debug {
}
impl<T: ToPon + Send + Debug> OutMessage for T {}

#[derive(Debug)]
pub struct RequestError {
    pub request_id: String,
    pub error_type: RequestErrorType,
    pub message: String
}

#[derive(Debug)]
pub enum RequestErrorType {
    BadRequest,
    InternalError,
}

pub type SocketToken = usize;
