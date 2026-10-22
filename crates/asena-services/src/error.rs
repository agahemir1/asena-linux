#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ServiceError(pub String);
impl From<String> for ServiceError {
    fn from(message: String) -> Self {
        Self(message)
    }
}
