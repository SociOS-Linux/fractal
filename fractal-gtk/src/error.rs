use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct StandardErrorResponse {
    pub errcode: String,
    pub error: String,
}

type MatrixErrorCode = String;

#[macro_export]
macro_rules! derror {
    ($from: path, $to: path) => {
        impl From<$from> for Error {
            fn from(_: $from) -> Error {
                $to
            }
        }
    };
}

#[derive(Debug)]
pub enum Error {
    BackendError,
    ReqwestError(fractal_api::reqwest::Error),
    NetworkError(fractal_api::reqwest::StatusCode),
    MatrixError(MatrixErrorCode, String),
}

impl From<fractal_api::reqwest::Error> for Error {
    fn from(err: fractal_api::reqwest::Error) -> Error {
        Error::ReqwestError(err)
    }
}

impl From<StandardErrorResponse> for Error {
    fn from(resp: StandardErrorResponse) -> Error {
        Error::MatrixError(resp.errcode, resp.error)
    }
}

derror!(glib::error::Error, Error::BackendError);
derror!(fractal_api::identifiers::Error, Error::BackendError);
