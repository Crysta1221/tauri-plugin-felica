use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

/// Classification of errors that can occur during FeliCa plugin operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FelicaErrorCode {
    DeviceDisconnected,
    DeviceNotFound,
    DeviceBusy,
    DeviceAccessDenied,
    UnsupportedDevice,
    ScanTimeout,
    ScanCancelled,
    CommunicationError,
    ProtocolError,
    UnsupportedCard,
    CardTypeMismatch,
    SessionClosed,
    InternalError,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Reader device was disconnected")]
    DeviceDisconnected,

    #[error("Reader device not found: {0}")]
    DeviceNotFound(String),

    #[error("Reader device is busy")]
    DeviceBusy,

    #[error("USB access denied: please verify driver configuration (WinUSB/libusb) - {0}")]
    DeviceAccessDenied(String),

    #[error("Reader does not support this operation: {0}")]
    UnsupportedDevice(String),

    #[error("Card scan timed out")]
    ScanTimeout,

    #[error("Card scan was cancelled")]
    ScanCancelled,

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Card is not supported: {message}")]
    UnsupportedCard {
        message: String,
        system_codes: Vec<u16>,
    },

    #[error("Card type does not match the requested targets")]
    CardTypeMismatch {
        system_codes: Vec<u16>,
        detected: Vec<String>,
    },

    #[error("Session '{0}' is closed or does not exist")]
    SessionClosed(String),

    #[error(transparent)]
    Rusb(#[from] rusb::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn unsupported_card(message: impl Into<String>, system_codes: Vec<u16>) -> Self {
        Self::UnsupportedCard {
            message: message.into(),
            system_codes,
        }
    }

    /// Returns the corresponding [`FelicaErrorCode`].
    pub fn error_code(&self) -> FelicaErrorCode {
        match self {
            Self::DeviceDisconnected => FelicaErrorCode::DeviceDisconnected,
            Self::DeviceNotFound(_) => FelicaErrorCode::DeviceNotFound,
            Self::DeviceBusy => FelicaErrorCode::DeviceBusy,
            Self::DeviceAccessDenied(_) => FelicaErrorCode::DeviceAccessDenied,
            Self::UnsupportedDevice(_) => FelicaErrorCode::UnsupportedDevice,
            Self::ScanTimeout => FelicaErrorCode::ScanTimeout,
            Self::ScanCancelled => FelicaErrorCode::ScanCancelled,
            Self::CommunicationError(_) => FelicaErrorCode::CommunicationError,
            Self::ProtocolError(_) => FelicaErrorCode::ProtocolError,
            Self::UnsupportedCard { .. } => FelicaErrorCode::UnsupportedCard,
            Self::CardTypeMismatch { .. } => FelicaErrorCode::CardTypeMismatch,
            Self::SessionClosed(_) => FelicaErrorCode::SessionClosed,
            Self::Rusb(err) => match err {
                rusb::Error::NoDevice | rusb::Error::NotFound => FelicaErrorCode::DeviceDisconnected,
                rusb::Error::Access => FelicaErrorCode::DeviceAccessDenied,
                rusb::Error::Busy => FelicaErrorCode::DeviceBusy,
                rusb::Error::Timeout => FelicaErrorCode::CommunicationError,
                _ => FelicaErrorCode::CommunicationError,
            },
            Self::Io(_) => FelicaErrorCode::InternalError,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FelicaErrorBody {
    code: FelicaErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_codes: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detected: Option<Vec<String>>,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (system_codes, detected) = match self {
            Self::UnsupportedCard { system_codes, .. } => {
                let codes = if system_codes.is_empty() {
                    None
                } else {
                    Some(system_codes.clone())
                };
                (codes, None)
            }
            Self::CardTypeMismatch {
                system_codes,
                detected,
            } => {
                let codes = if system_codes.is_empty() {
                    None
                } else {
                    Some(system_codes.clone())
                };
                (codes, Some(detected.clone()))
            }
            _ => (None, None),
        };

        FelicaErrorBody {
            code: self.error_code(),
            message: self.to_string(),
            system_codes,
            detected,
        }
        .serialize(serializer)
    }
}
