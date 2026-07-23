use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for Storable trait
///
/// Usage:
/// ```ignore
///   #[derive(Serialize, Storable)]
///   #[storable(key = "users")]
///   struct UserCount(usize);
/// ```
#[proc_macro_derive(Storable, attributes(storable))]
pub fn derive_storable(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;

  let key = input
    .attrs
    .iter()
    .find(|attr| attr.path().is_ident("storable"))
    .and_then(|attr| {
      let mut key = None;
      attr
        .parse_nested_meta(|meta| {
          if meta.path.is_ident("key") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            key = Some(s.value());
            Ok(())
          } else {
            Err(meta.error("unsupported attribute"))
          }
        })
        .ok()?;
      key
    })
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
  s.split('_')
    .map(|word| {
      let mut chars = word.chars();
      match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
      }
    })
    .collect()
}

#[proc_macro_derive(FrontboxEvent)]
pub fn derive_frontbox_event(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;

  let expanded = quote! {
      impl FrontboxEvent for #name {
          fn as_any(&self) -> &dyn Any {
              self
          }
      }
  };

  TokenStream::from(expanded)
}
