//! Token estimation and cost calculation utilities

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Model pricing per 1M tokens (input, output) in USD
static MODEL_PRICING: Lazy<HashMap<&'static str, (f64, f64)>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Claude models
    m.insert("claude-3-opus", (15.0, 75.0));
    m.insert("claude-3-sonnet", (3.0, 15.0));
    m.insert("claude-3-haiku", (0.25, 1.25));
    m.insert("claude-3.5-sonnet", (3.0, 15.0));
    m.insert("claude-3.5-haiku", (0.8, 4.0));
    m.insert("claude-sonnet-4", (3.0, 15.0));
    m.insert("claude-opus-4", (15.0, 75.0));

    // OpenAI models
    m.insert("gpt-4", (30.0, 60.0));
    m.insert("gpt-4-turbo", (10.0, 30.0));
    m.insert("gpt-4o", (2.5, 10.0));
    m.insert("gpt-4o-mini", (0.15, 0.6));
    m.insert("gpt-3.5-turbo", (0.5, 1.5));
    m.insert("o1", (15.0, 60.0));
    m.insert("o1-mini", (3.0, 12.0));

    // Google models
    m.insert("gemini-pro", (0.5, 1.5));
    m.insert("gemini-1.5-pro", (1.25, 5.0));
    m.insert("gemini-1.5-flash", (0.075, 0.3));
    m.insert("gemini-2.0-flash", (0.1, 0.4));

    // Default
    m.insert("default", (1.0, 3.0));

    m
});

/// Estimate token count from character count
/// Rule of thumb: ~4 characters per token for English text/code
pub fn estimate_tokens_from_chars(chars: usize) -> u64 {
    (chars / 4) as u64
}

/// Estimate token count from word count
/// Rule of thumb: ~0.75 tokens per word for English
#[allow(dead_code)]
pub fn estimate_tokens_from_words(words: usize) -> u64 {
    ((words as f64) * 0.75).ceil() as u64
}

/// Estimate token count from line count (code)
/// Rule of thumb: ~10-15 tokens per line of code
#[allow(dead_code)]
pub fn estimate_tokens_from_lines(lines: usize) -> u64 {
    (lines * 12) as u64
}

/// Estimate tokens from request count
/// Average request: ~500 tokens (very rough estimate)
#[allow(dead_code)]
pub fn estimate_tokens_from_requests(requests: u64) -> u64 {
    requests * 500
}

/// Find pricing for a model (fuzzy match)
fn find_model_pricing(model: Option<&str>) -> (f64, f64) {
    let model = match model {
        Some(m) => m.to_lowercase(),
        None => return *MODEL_PRICING.get("default").unwrap(),
    };

    // Try exact match first
    if let Some(pricing) = MODEL_PRICING.get(model.as_str()) {
        return *pricing;
    }

    // Try partial match
    for (key, pricing) in MODEL_PRICING.iter() {
        if model.contains(key) || key.contains(&model.as_str()) {
            return *pricing;
        }
    }

    *MODEL_PRICING.get("default").unwrap()
}

/// Calculate cost based on token usage and model
pub fn calculate_cost(input_tokens: u64, output_tokens: u64, model: Option<&str>) -> f64 {
    let (input_price, output_price) = find_model_pricing(model);

    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

    input_cost + output_cost
}

/// Calculate cache cost savings
/// Cache reads are typically 10% of input cost
#[allow(dead_code)]
pub fn calculate_cache_savings(cache_read_tokens: u64, model: Option<&str>) -> f64 {
    let (input_price, _) = find_model_pricing(model);
    let full_cost = (cache_read_tokens as f64 / 1_000_000.0) * input_price;
    let cache_cost = full_cost * 0.1; // Cache reads cost 10% of input
    full_cost - cache_cost
}
