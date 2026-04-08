//! Cognate Prompts — type-safe prompt templating with compile-time validation.
//!
//! # Overview
//!
//! 1. Define a struct holding your prompt variables.
//! 2. Derive [`Prompt`] and annotate it with `#[template("…")]`.
//! 3. Call `.render()` at runtime to produce the filled-in string.
//!
//! Template syntax uses Handlebars `{{field_name}}` placeholders.  Unknown
//! variable names are a **compile-time error**.
//!
//! # Example
//!
//! ```rust
//! use cognate_prompts::Prompt;
//! use serde::Serialize;
//!
//! #[derive(Prompt, Serialize)]
//! #[template("Hello {{name}}, you are {{age}} years old.")]
//! struct Greeting {
//!     name: String,
//!     age: u32,
//! }
//!
//! let g = Greeting { name: "Alice".to_string(), age: 30 };
//! assert_eq!(g.render().unwrap(), "Hello Alice, you are 30 years old.");
//! ```
#![warn(missing_docs)]

pub use cognate_prompts_derive::Prompt;
use handlebars::Handlebars;
use serde::Serialize;

/// A type-safe, compile-time validated prompt template.
///
/// Derive this trait with `#[derive(Prompt)]` and supply a
/// `#[template("…")]` attribute.  See the [crate documentation](crate) for
/// a full example.
pub trait Prompt: Serialize {
    /// Render the template by substituting the struct's fields.
    ///
    /// # Errors
    ///
    /// Returns an error if Handlebars fails to render (e.g. a value is not
    /// JSON-serialisable).  Structural template errors (unknown variables)
    /// are caught at compile time, not here.
    fn render(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Return the raw Handlebars template string.
    fn template() -> &'static str
    where
        Self: Sized;
}

/// Render `template` with `data` using Handlebars.
///
/// This is the runtime function called by the `#[derive(Prompt)]`-generated
/// `render` implementation.
pub fn render_template(
    template: &str,
    data: &impl Serialize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut hb = Handlebars::new();
    hb.set_strict_mode(true);
    hb.register_template_string("t", template)?;
    Ok(hb.render("t", data)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Simple {
        name: String,
        age: u32,
    }

    #[test]
    fn test_render_template() {
        let data = Simple {
            name: "Bob".to_string(),
            age: 25,
        };
        let result = render_template("Hi {{name}}, age {{age}}", &data).unwrap();
        assert_eq!(result, "Hi Bob, age 25");
    }
}
