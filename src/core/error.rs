// src/core/error.rs

use std::time::SystemTimeError;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug)]
pub enum InfernoError {
    Network(String),
    ParseError(String),
    NoiseError(String),
}

impl From<libp2p::multiaddr::Error> for InfernoError {
    fn from(err: libp2p::multiaddr::Error) -> Self {
        InfernoError::Network(err.to_string())
    }
}

impl From<libp2p::TransportError<std::io::Error>> for InfernoError {
    fn from(err: libp2p::TransportError<std::io::Error>) -> Self {
        InfernoError::Network(err.to_string())
    }
}

impl From<libp2p::swarm::DialError> for InfernoError {
    fn from(err: libp2p::swarm::DialError) -> Self {
        InfernoError::Network(err.to_string())
    }
}

impl From<libp2p::noise::Error> for InfernoError {
    fn from(err: libp2p::noise::Error) -> Self {
        InfernoError::NoiseError(err.to_string())
    }
}

impl From<std::convert::Infallible> for InfernoError {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!("Infallible errors should never occur")
    }
}

impl From<String> for InfernoError {
    fn from(err: String) -> Self {
        InfernoError::Network(err)
    }
}

impl From<SystemTimeError> for InfernoError {
    fn from(err: SystemTimeError) -> Self {
        InfernoError::Network(format!("System time error: {}", err))
    }
}

impl<T> From<SendError<T>> for InfernoError {
    fn from(err: SendError<T>) -> Self {
        InfernoError::Network(format!("Tokio send error: {}", err))
    }
}