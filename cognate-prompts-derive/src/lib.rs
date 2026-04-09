//! Procedural macros for `cognate-prompts`.
//!
//! This crate is an implementation detail — use `cognate_prompts::Prompt` instead.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

/// Derive the `cognate_prompts::Prompt` trait for a struct.
///
/// # Attributes
///
/// * `#[template("…")]` — *required* — a Handlebars template string.
///   Variables are written as `{{field_name}}`.
///
/// # Compile-time validation
///
/// Every `{{variable}}` in the template is checked against the struct's field
/// names at compile time.  Using an unknown variable is a **compile error**.
///
/// # Example
///
/// ```rust,ignore
/// use cognate_prompts::Prompt;
/// use serde::Serialize;
///
/// #[derive(Prompt, Serialize)]
/// #[template("Summarise the following text by {{author}}: {{text}}")]
/// struct SummarisePrompt {
///     author: String,
///     text: String,
/// }
///
/// let prompt = SummarisePrompt {
///     author: "Alice".to_string(),
///     text: "Rust is fast and reliable.".to_string(),
/// };
///
/// assert_eq!(
///     prompt.render().unwrap(),
///     "Summarise the following text by Alice: Rust is fast and reliable."
/// );
/// ```
#[proc_macro_derive(Prompt, attributes(template))]
pub fn derive_prompt(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // ── Parse #[template("…")] ─────────────────────────────────────────────
    let mut template = String::new();
    let mut template_span = Span::call_site();

    for attr in &input.attrs {
        if attr.path().is_ident("template") {
            match attr.parse_args::<LitStr>() {
                Ok(s) => {
                    template_span = s.span();
                    template = s.value();
                }
                Err(e) => return e.into_compile_error().into(),
            }
        }
    }

    if template.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "#[derive(Prompt)] requires a template: add `#[template(\"…\")]`",
        )
        .into_compile_error()
        .into();
    }

    // ── Only structs are supported ─────────────────────────────────────────
    let fields: std::collections::HashSet<String> = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
                .collect(),
            _ => {
                return syn::Error::new(
                    Span::call_site(),
                    "#[derive(Prompt)] requires a struct with named fields",
                )
                .into_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "#[derive(Prompt)] can only be applied to structs",
            )
            .into_compile_error()
            .into();
        }
    };

    // ── Validate template variables ────────────────────────────────────────
    // Scan for `{{variable}}` patterns using simple string searching.
    // We handle `{{` as opening and `}}` as closing, skipping `{{{…}}}` (Handlebars
    // triple-stash) by requiring exact double-brace pairs.
    let mut pos = 0;
    while let Some(open_rel) = template[pos..].find("{{") {
        let open = pos + open_rel + 2; // position just after `{{`

        // Skip Handlebars triple-stash `{{{`
        if template.as_bytes().get(open) == Some(&b'{') {
            pos = open + 1;
            continue;
        }

        match template[open..].find("}}") {
            Some(close_rel) => {
                let var = template[open..open + close_rel].trim();
                if !var.is_empty() && !fields.contains(var) {
                    return syn::Error::new(
                        template_span,
                        format!(
                            "template variable `{{{{{}}}}}` does not match any field of `{}`",
                            var, name
                        ),
                    )
                    .into_compile_error()
                    .into();
                }
                pos = open + close_rel + 2;
            }
            None => {
                return syn::Error::new(
                    template_span,
                    "unclosed `{{` in template — every `{{` must have a matching `}}`",
                )
                .into_compile_error()
                .into();
            }
        }
    }

    // ── Code generation ────────────────────────────────────────────────────
    let expanded = quote! {
        impl cognate_prompts::Prompt for #name {
            fn render(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                cognate_prompts::render_template(Self::template(), self)
            }

            fn template() -> &'static str {
                #template
            }
        }
    };

    TokenStream::from(expanded)
}
