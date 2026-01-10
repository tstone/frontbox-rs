pub mod id;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum FastResponse {
  Failed(String),
  IdResponse {
    processor: String,
    product_number: String,
    firmware_version: String,
  },
}
