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

/// Turn any object in a Frontbox Event. Child data must implement serde::Serialize
///
/// A generic event is one for each of its type parameters, which must be `'static`:
/// ```ignore
///   #[derive(Serialize, Event)]
///   struct Ejected<S> {
///     #[serde(skip)]
///     scoop: PhantomData<fn() -> S>,
///   }
/// ```
#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

  let mut where_clause = input
    .generics
    .where_clause
    .clone()
    .unwrap_or_else(|| syn::parse_quote!(where));
  where_clause
    .predicates
    .push(syn::parse_quote!(Self: serde::Serialize + Send + Sync + 'static));

  let expanded = quote! {
      impl #impl_generics frontbox_core::systems::Event for #name #ty_generics #where_clause {
          fn as_any(&self) -> &dyn std::any::Any {
              self
          }
          fn to_json(&self) -> Option<serde_json::Value> {
              serde_json::to_value(self).ok()
          }
      }
  };

  TokenStream::from(expanded)
}

/// Turn any object in a Frontbox State. Must also be `Serialize + Default + Clone + PartialEq`
#[proc_macro_derive(State)]
pub fn derive_state(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;

  let expanded = quote! {
      impl frontbox_core::systems::State for #name {}
  };

  TokenStream::from(expanded)
}

/// Turn any object in a Frontbox Tag
#[proc_macro_derive(Tag)]
pub fn derive_tag(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let name_str = name.to_string();

  let expanded = quote! {
      impl frontbox_core::hardware::Tag for #name {
          fn as_any(&self) -> &dyn std::any::Any { self }
          fn tag_name(&self) -> &'static str { #name_str }
      }

      impl std::fmt::Debug for #name {
          fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
              f.write_str(#name_str)
          }
      }

      impl Clone for #name {
          fn clone(&self) -> Self { #name }
      }
  };

  TokenStream::from(expanded)
}
