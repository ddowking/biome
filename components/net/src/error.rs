// Copyright:: Copyright (c) 2015-2016 The Habitat Maintainers
//
// The terms of the Evaluation Agreement (Habitat) between Chef Software Inc.
// and the party accessing this file ("Licensee") apply to Licensee's use of
// the Software until such time that the Software is made available under an
// open source license such as the Apache 2.0 License.

use std::error;
use std::fmt;
use std::io;
use std::result;

use protobuf;
use zmq;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    MaxHops,
    Protobuf(protobuf::ProtobufError),
    Sys,
    Zmq(zmq::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::IO(ref e) => format!("{}", e),
            Error::MaxHops => format!("Received a message containing too many network hops"),
            Error::Protobuf(ref e) => format!("{}", e),
            Error::Sys => format!("Internal system error"),
            Error::Zmq(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IO(ref err) => err.description(),
            Error::MaxHops => "Received a message containing too many network hops",
            Error::Protobuf(ref err) => err.description(),
            Error::Sys => "Internal system error",
            Error::Zmq(ref err) => err.description(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error {
        Error::Protobuf(err)
    }
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Error {
        Error::Zmq(err)
    }
}
