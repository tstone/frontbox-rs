#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawResponse {
  pub prefix: String,
  pub address: Option<String>,
  pub payload: String,
}
