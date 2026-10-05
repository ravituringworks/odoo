use thiserror::Error;

#[derive(Error, Debug)]
pub enum OdooError {
    #[error("unknown model `{0}`")] UnknownModel(String),
    #[error("unknown field `{1}` on model `{0}`")] UnknownField(String, String),
    #[error("access denied: {op} on {model} for user {uid}")] AccessDenied { op: String, model: String, uid: i64 },
    #[error("validation error: {0}")] Validation(String),
    #[error("missing required field `{1}` on `{0}`")] Required(String, String),
    #[error("record not found: {0}[{1}]")] NotFound(String, i64),
    #[error("invalid domain: {0}")] Domain(String),
    #[error("storage error: {0}")] Storage(String),
    #[error("{0}")] User(String),
}
pub type Result<T> = std::result::Result<T, OdooError>;
