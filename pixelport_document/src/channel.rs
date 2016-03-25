
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

pub type SocketToken = usize; // This is either a tcp socket or a c process

#[derive(Debug)]
pub struct OutgoingMessage {
    pub channel_id: String, // Either a request_id or a stream_id
    pub socket_token: SocketToken,
    pub message: Result<Box<OutMessage>, RequestError>
}
impl OutgoingMessage {
    pub fn to_tcpmessage(&self) -> String {
        match &self.message {
            &Ok(ref message) => format!("{} ok {}", self.channel_id, message.to_pon().to_string()),
            &Err(ref err) => format!("{} err {:?}", self.channel_id, err),
        }
    }
}
