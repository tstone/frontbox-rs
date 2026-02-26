use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for Storable trait
///
/// Usage:
/// ```rust
///   #[derive(Serialize, Storable)]
///   #[storable(key = "users")]
///   struct UserCount(usize);
/// ```
#[proc_macro_derive(Storable)]
pub fn derive_storable(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;

  // Check for #[storable(key = "custom_key")]
  let key = input
    .attrs
    .iter()
    .find(|attr| attr.path().is_ident("storable"))
    .and_then(|attr| attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value()))
    .unwrap_or_else(|| to_camel_case(&name.to_string()));

  let expanded = quote! {
      impl Storable for #name {
          fn to_json(&self) -> serde_json::Value {
              serde_json::to_value(self).unwrap()
          }

          fn key(&self) -> &str {
              #key
          }
      }
  };

  TokenStream::from(expanded)
}

fn to_camel_case(s: &str) -> String {
  s.to_string()
    .split('_')
    .map(|word| {
      let mut chars = word.chars();
      match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
      }
    })
    .collect()
}
