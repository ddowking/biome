use crate::{bio_core,
            bio_http};
use reqwest;
use serde_json;
use std::{error,
          fmt,
          io,
          num,
          path::PathBuf,
          result};
use tokio::task::JoinError;
use url;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    APIError(reqwest::StatusCode, String),
    BadResponseBody(reqwest::Error),
    DownloadWrite(PathBuf, io::Error),
    BiomeCore(bio_core::Error),
    BiomeHttpClient(bio_http::Error),
    ReqwestError(reqwest::Error),
    IO(io::Error),
    Json(serde_json::Error),
    KeyReadError(PathBuf, io::Error),
    MissingHeader(String),
    InvalidHeader(String),
    NoFilePart,
    PackageReadError(PathBuf, io::Error),
    ParseIntError(num::ParseIntError),
    IdentNotFullyQualified,
    UploadFailed(String),
    UrlParseError(url::ParseError),
    WriteSyncFailed,
    NotSupported,
    TokioJoinError(JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match *self {
            Error::APIError(ref c, ref m) if !m.is_empty() => format!("[{}] {}", c, m),
            Error::APIError(ref c, _) => format!("[{}]", c),
            Error::BadResponseBody(ref e) => format!("Failed to read response body, {}", e),
            Error::DownloadWrite(ref p, ref e) => {
                format!("Failed to write contents of builder response, {}, {}",
                        p.display(),
                        e)
            }
            Error::BiomeCore(ref e) => format!("{}", e),
            Error::BiomeHttpClient(ref e) => format!("{}", e),
            Error::ReqwestError(ref err) => format!("{}", err),
            Error::IO(ref e) => format!("{}", e),
            Error::Json(ref e) => format!("{}", e),
            Error::KeyReadError(ref p, ref e) => {
                format!("Failed to read origin key, {}, {}", p.display(), e)
            }
            Error::MissingHeader(ref s) => format!("Response is missing a required header: {}", s),
            Error::InvalidHeader(ref s) => format!("Response header is invalid: {}", s),
            Error::NoFilePart => "An invalid path was passed - we needed a filename, and this \
                                  path does not have one"
                                                         .to_string(),
            Error::PackageReadError(ref p, ref e) => {
                format!("Failed to read package artifact, {}, {}", p.display(), e)
            }
            Error::ParseIntError(ref err) => format!("{}", err),
            Error::IdentNotFullyQualified => {
                "Cannot perform the specified operation. Specify a fully qualifed package \
                 identifier (ex: core/busybox-static/1.42.2/20170513215502)"
                                                                            .to_string()
            }
            Error::UploadFailed(ref s) => format!("Upload failed: {}", s),
            Error::UrlParseError(ref e) => format!("{}", e),
            Error::WriteSyncFailed => {
                "Could not write to destination; perhaps the disk is full?".to_string()
            }
            Error::NotSupported => "The specified operation is not supported.".to_string(),
            Error::TokioJoinError(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {}

impl From<bio_core::Error> for Error {
    fn from(err: bio_core::Error) -> Error { Error::BiomeCore(err) }
}

impl From<bio_http::Error> for Error {
    fn from(err: bio_http::Error) -> Error { Error::BiomeHttpClient(err) }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error { Error::ReqwestError(err) }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error { Error::IO(err) }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error { Error::Json(err) }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Error { Error::UrlParseError(err) }
}

impl From<JoinError> for Error {
    fn from(err: JoinError) -> Error { Error::TokioJoinError(err) }
}
