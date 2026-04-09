//! Procedural macros for `cognate-tools`.
//!
//! This crate is an implementation detail — use `cognate_tools::Tool` instead.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

/// Derive the `cognate_tools::Tool` trait for a struct.
///
/// # Attributes
///
/// * `#[tool(description = "…")]` — *required* — sets the tool description
///   shown to the model.
/// * `#[tool_param(description = "…")]` — *optional*, on individual fields —
///   documents the field; schemars picks up `///` doc comments automatically,
///   but this attribute is also accepted for explicitness.
///
/// # Requirements
///
/// The struct must:
///
/// 1. Also derive `serde::Serialize`, `serde::Deserialize`, and
///    `schemars::JsonSchema` so the macro can generate a JSON Schema and
///    deserialise arguments at runtime.
/// 2. Provide an `async fn run(&self) -> Result<T, E>` method where
///    `T: serde::Serialize` and `E: Into<Box<dyn std::error::Error + Send + Sync>>`.
///    The derive macro calls this method; if it is missing you will get a
///    compile error pointing to the generated `call` implementation.
///
/// # Example
///
/// ```rust,ignore
/// use cognate_tools::Tool;
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
///
/// #[derive(Tool, Serialize, Deserialize, JsonSchema)]
/// #[tool(description = "Look up the current weather for a city")]
/// struct GetWeather {
///     /// Name of the city to look up.
///     city: String,
/// }
///
/// impl GetWeather {
///     async fn run(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
///         Ok(format!("Sunny, 22 °C in {}", self.city))
///     }
/// }
/// ```
#[proc_macro_derive(Tool, attributes(tool, tool_param))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    // ── Parse #[tool(description = "…")] ──────────────────────────────────
    let mut description = String::new();
    for attr in &input.attrs {
        if attr.path().is_ident("tool") {
            let result = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    description = s.value();
                }
                Ok(())
            });
            if let Err(e) = result {
                return e.into_compile_error().into();
            }
        }
    }

    if description.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[derive(Tool)] requires a description: add `#[tool(description = \"…\")]`",
        )
        .into_compile_error()
        .into();
    }

    // ── Validate #[tool_param(description = "…")] on fields ───────────────
    if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            for field in &fields.named {
                for attr in &field.attrs {
                    if attr.path().is_ident("tool_param") {
                        if let Err(e) = attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("description") {
                                let value = meta.value()?;
                                let _: LitStr = value.parse()?;
                            }
                            Ok(())
                        }) {
                            return e.into_compile_error().into();
                        }
                    }
                }
            }
        }
    } else {
        return syn::Error::new(
            Span::call_site(),
            "#[derive(Tool)] can only be applied to structs",
        )
        .into_compile_error()
        .into();
    }

    // ── Code generation ────────────────────────────────────────────────────
    let expanded = quote! {
        #[async_trait::async_trait]
        impl cognate_tools::Tool for #name {
            fn name(&self) -> &str {
                #name_str
            }

            fn description(&self) -> &str {
                #description
            }

            fn parameters(&self) -> serde_json::Value {
                let schema = schemars::schema_for!(#name);
                serde_json::to_value(schema).unwrap_or_default()
            }

            async fn call(
                &self,
                params: serde_json::Value,
            ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
                let args: #name = serde_json::from_value(params)?;
                let result = args.run().await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    };

    TokenStream::from(expanded)
}
