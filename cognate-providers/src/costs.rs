//! Model pricing constants.
//!
//! All prices are expressed as USD per **1 000 tokens**.
//!
//! # Usage
//!
//! ```rust
//! use cognate_providers::costs;
//! use cognate_core::Usage;
//!
//! let usage = Usage { prompt_tokens: 500, completion_tokens: 150, total_tokens: 650 };
//! let cost = usage.calculate_cost(
//!     costs::GPT4O.prompt_per_1k,
//!     costs::GPT4O.completion_per_1k,
//! );
//! println!("Cost: ${:.6}", cost);
//! ```

/// Pricing for a specific model, in USD per 1 000 tokens.
#[derive(Debug, Clone, Copy)]
pub struct ModelCost {
    /// Price per 1 000 prompt (input) tokens.
    pub prompt_per_1k: f64,
    /// Price per 1 000 completion (output) tokens.
    pub completion_per_1k: f64,
}

impl ModelCost {
    /// Calculate the total USD cost for a given token count.
    pub fn calculate(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        (prompt_tokens as f64 / 1000.0) * self.prompt_per_1k
            + (completion_tokens as f64 / 1000.0) * self.completion_per_1k
    }
}

// ─── OpenAI ────────────────────────────────────────────────────────────────

/// GPT-4o (gpt-4o)
pub const GPT4O: ModelCost = ModelCost {
    prompt_per_1k: 0.0025,
    completion_per_1k: 0.010,
};

/// GPT-4o mini (gpt-4o-mini)
pub const GPT4O_MINI: ModelCost = ModelCost {
    prompt_per_1k: 0.000150,
    completion_per_1k: 0.000600,
};

/// GPT-3.5 Turbo (gpt-3.5-turbo)
pub const GPT35_TURBO: ModelCost = ModelCost {
    prompt_per_1k: 0.0005,
    completion_per_1k: 0.0015,
};

/// GPT-4 Turbo (gpt-4-turbo)
pub const GPT4_TURBO: ModelCost = ModelCost {
    prompt_per_1k: 0.010,
    completion_per_1k: 0.030,
};

// ─── Anthropic ─────────────────────────────────────────────────────────────

/// Claude 3.5 Sonnet (claude-3-5-sonnet-20241022)
pub const CLAUDE_35_SONNET: ModelCost = ModelCost {
    prompt_per_1k: 0.003,
    completion_per_1k: 0.015,
};

/// Claude 3.5 Haiku (claude-3-5-haiku-20241022)
pub const CLAUDE_35_HAIKU: ModelCost = ModelCost {
    prompt_per_1k: 0.0008,
    completion_per_1k: 0.004,
};

/// Claude 3 Opus (claude-3-opus-20240229)
pub const CLAUDE_3_OPUS: ModelCost = ModelCost {
    prompt_per_1k: 0.015,
    completion_per_1k: 0.075,
};

/// Claude 3 Haiku (claude-3-haiku-20240307)
pub const CLAUDE_3_HAIKU: ModelCost = ModelCost {
    prompt_per_1k: 0.00025,
    completion_per_1k: 0.00125,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let cost = GPT4O_MINI.calculate(1000, 500);
        // 1000 prompt * 0.000150/1k + 500 completion * 0.000600/1k
        let expected = 0.000150 + 0.000300;
        assert!((cost - expected).abs() < 1e-9);
    }
}
