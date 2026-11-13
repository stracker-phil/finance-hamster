//! The app's error type and its JSON shape across the Tauri boundary (`{ kind, message, field? }`, ADR 0003).
//! Features add kinds as the UI needs to tell their failures apart.

use serde::{Deserialize, Serialize, Serializer};
use ts_rs::TS;

pub type AppResult<T> = Result<T, AppError>;

/// Every error that crosses the Tauri boundary.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("File error: {0}")]
    Io(String),
    #[error("{0}")]
    InvalidInput(String),
    /// Invalid input in one form field; `field` is its path (`account.iban`).
    #[error("`{field}`: {message}")]
    InvalidField { field: String, message: String },
    #[error("{0}")]
    NotFound(String),
    /// A bug or a broken invariant, not a user mistake.
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            AppError::Io(_) => ErrorKind::Io,
            AppError::InvalidInput(_) | AppError::InvalidField { .. } => ErrorKind::InvalidInput,
            AppError::NotFound(_) => ErrorKind::NotFound,
            AppError::Internal(_) => ErrorKind::Internal,
        }
    }

    pub fn field(&self) -> Option<FieldJson<'_>> {
        match self {
            AppError::InvalidField { field, message } => Some(FieldJson {
                path: field,
                message,
            }),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorJson {
            kind: self.kind(),
            message: self.to_string(),
            field: self.field(),
        }
        .serialize(serializer)
    }
}

/// The JSON shape of an `AppError`.
#[derive(Serialize, TS)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "AppError")]
struct ErrorJson<'a> {
    kind: ErrorKind,
    message: String,
    /// The form field the error is about (`InvalidField` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    field: Option<FieldJson<'a>>,
}

/// `message` is the error's message without the `` `path`: `` prefix.
#[derive(Serialize, TS, Debug, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "AppErrorField")]
pub struct FieldJson<'a> {
    pub path: &'a str,
    pub message: &'a str,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Io,
    InvalidInput,
    NotFound,
    Internal,
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::InvalidInput(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_kind_and_message() {
        assert_eq!(
            serde_json::to_value(AppError::NotFound("Statement 3".into())).unwrap(),
            json!({ "kind": "not_found", "message": "Statement 3" })
        );
    }

    #[test]
    fn invalid_field_serializes_its_path_and_message_without_prefix() {
        let err = AppError::InvalidField {
            field: "account.iban".into(),
            message: "must not be empty".into(),
        };
        assert_eq!(
            serde_json::to_value(&err).unwrap(),
            json!({
                "kind": "invalid_input",
                "message": "`account.iban`: must not be empty",
                "field": { "path": "account.iban", "message": "must not be empty" }
            })
        );
    }

    #[test]
    fn field_is_present_only_for_invalid_field() {
        let err = AppError::InvalidField {
            field: "a".into(),
            message: "b".into(),
        };
        assert_eq!(
            err.field(),
            Some(FieldJson {
                path: "a",
                message: "b"
            })
        );
        assert_eq!(AppError::InvalidInput("x".into()).field(), None);
        assert_eq!(
            serde_json::to_value(AppError::Internal("broken".into())).unwrap(),
            json!({ "kind": "internal", "message": "broken" })
        );
    }

    #[test]
    fn every_variant_has_its_kind() {
        let cases = [
            (AppError::Io(String::new()), ErrorKind::Io),
            (
                AppError::InvalidInput(String::new()),
                ErrorKind::InvalidInput,
            ),
            (
                AppError::InvalidField {
                    field: "a".into(),
                    message: "b".into(),
                },
                ErrorKind::InvalidInput,
            ),
            (AppError::NotFound(String::new()), ErrorKind::NotFound),
            (AppError::Internal(String::new()), ErrorKind::Internal),
        ];
        for (err, kind) in cases {
            assert_eq!(err.kind(), kind, "{err:?}");
        }
    }

    #[test]
    fn kinds_serialize_in_snake_case() {
        for (kind, wire) in [
            (ErrorKind::Io, "io"),
            (ErrorKind::InvalidInput, "invalid_input"),
            (ErrorKind::NotFound, "not_found"),
            (ErrorKind::Internal, "internal"),
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(wire), "{wire}");
        }
    }

    #[test]
    fn io_errors_convert_to_io_with_a_file_error_message() {
        let err = AppError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert_eq!(err, AppError::Io("denied".into()));
        assert_eq!(err.to_string(), "File error: denied");
    }

    #[test]
    fn json_errors_convert_to_invalid_input() {
        let parse_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let message = parse_error.to_string();
        let err = AppError::from(parse_error);
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), message);
    }
}
