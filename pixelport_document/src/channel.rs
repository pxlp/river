
use std::fmt::Debug;
use pon::*;
use bus::*;
use pon_translater::*;

pub trait OutMessage: ToPon + Send + Debug {
}
impl<T: ToPon + Send + Debug> OutMessage for T {}

#[derive(Debug)]
pub struct RequestError {
    pub error_type: RequestErrorType,
    pub message: String
}

#[derive(Debug)]
pub enum RequestErrorType {
    BadRequest,
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientId {
    SocketToken(usize),
    CAPI,
    Broadcast
}

// This is either a request_id or a stream_id
pub type ChannelId = String;

#[derive(Debug)]
pub struct IncomingMessage {
    pub client_id: ClientId,
    pub channel_id: ChannelId,
    pub message: Box<BusValue>
}
impl IncomingMessage {
    pub fn from_tcpstring(translater: &PonTranslater, bus: &mut Bus, client_id: ClientId, message: &str) -> Result<IncomingMessage, OutgoingMessage> {
        let split: Vec<&str> = message.splitn(2, " ").collect();
        if split.len() != 2 {
            return Err(OutgoingMessage {
                channel_id: "unknown".to_string(),
                client_id: client_id,
                message: Err(RequestError {
                    error_type: RequestErrorType::BadRequest,
                    message: "Expected format: <channel_id> <message>".to_string()
                })
            });
        }
        let channel_id = split[0].to_string();
        IncomingMessage::from_string(translater, bus, client_id, channel_id, &split[1])
    }
    pub fn from_string(translater: &PonTranslater, bus: &mut Bus, client_id: ClientId, channel_id: ChannelId, message: &str) -> Result<IncomingMessage, OutgoingMessage> {
        match Pon::from_string(message) {
            Ok(pon) => match translater.translate_raw(&pon, bus) {
                Ok(message) => Ok(IncomingMessage {
                    channel_id: channel_id,
                    client_id: client_id,
                    message: message
                }),
                Err(err) => Err(OutgoingMessage {
                    channel_id: channel_id,
                    client_id: client_id,
                    message: Err(RequestError {
                        error_type: RequestErrorType::BadRequest,
                        message: format!("Unable to translate request: {:?}", err)
                    })
                })
            },
            Err(err) => Err(OutgoingMessage {
                channel_id: channel_id,
                client_id: client_id,
                message: Err(RequestError {
                    error_type: RequestErrorType::BadRequest,
                    message: format!("Unable to parse request: {:?}", err)
                })
            })
        }
    }
    pub fn ok<T: OutMessage + 'static>(&self, response: T) -> OutgoingMessage {
        OutgoingMessage {
            channel_id: self.channel_id.to_string(),
            client_id: self.client_id.clone(),
            message: Ok(Box::new(response))
        }
    }
    pub fn error(&self, error_type: RequestErrorType, message: &str) -> OutgoingMessage {
        OutgoingMessage {
            channel_id: self.channel_id.to_string(),
            client_id: self.client_id.clone(),
            message: Err(RequestError {
                error_type: error_type,
                message: message.to_string()
            })
        }
    }
    pub fn bad_request(&self, message: &str) -> OutgoingMessage {
        self.error(RequestErrorType::BadRequest, message)
    }
    pub fn internal_error(&self, message: &str) -> OutgoingMessage {
        self.error(RequestErrorType::InternalError, message)
    }
}

#[derive(Debug)]
pub struct OutgoingMessage {
    pub channel_id: ChannelId,
    pub client_id: ClientId,
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
