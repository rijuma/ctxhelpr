/// Byte-based token budget approximation.
///
/// Claude averages ~4 bytes per token, so `max_bytes = max_tokens * 4`.
/// This avoids a heavy BPE tokenizer dependency while being serviceable.
pub struct TokenBudget {
    max_bytes: usize,
}

impl TokenBudget {
    pub fn from_tokens(max_tokens: usize) -> Self {
        Self {
            max_bytes: max_tokens * 4,
        }
    }

    pub fn fits(&self, text: &str) -> bool {
        text.len() <= self.max_bytes
    }

    /// Truncate a JSON string to fit within budget.
    /// Returns the original if it fits, otherwise truncates the JSON array
    /// and appends a truncation marker.
    pub fn truncate_json(&self, text: &str, array_key: &str) -> String {
        if self.fits(text) {
            return text.to_string();
        }

        // Parse as JSON, progressively remove array items
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else {
            return self.truncate_raw(text);
        };

        let original_len = value
            .get(array_key)
            .and_then(|v| v.as_array())
            .map(|a| a.len());

        if let Some(total) = original_len {
            // Reserve space for truncation markers: ,"truncated":true,"total":99999
            let marker_overhead = 40;

            loop {
                let current_len = value[array_key].as_array().map(|a| a.len()).unwrap_or(0);
                let is_truncated = current_len < total;
                let budget = if is_truncated {
                    self.max_bytes.saturating_sub(marker_overhead)
                } else {
                    self.max_bytes
                };

                let candidate = value.to_string();
                if candidate.len() <= budget {
                    if is_truncated {
                        value["truncated"] = serde_json::json!(true);
                        value["total"] = serde_json::json!(total);
                    }
                    return value.to_string();
                }
                let arr = value.get_mut(array_key).and_then(|v| v.as_array_mut());
                match arr {
                    Some(a) if !a.is_empty() => {
                        a.pop();
                    }
                    _ => break,
                }
            }
        }

        // Fallback: raw truncation
        value["truncated"] = serde_json::json!(true);
        let result = value.to_string();
        if self.fits(&result) {
            return result;
        }
        self.truncate_raw(text)
    }

    fn truncate_raw(&self, text: &str) -> String {
        if self.max_bytes < 30 {
            return r#"{"truncated":true}"#.to_string();
        }
        let suffix = r#","truncated":true}"#;
        let available = self.max_bytes - suffix.len() - 1; // -1 for safety
        let boundary = super::floor_char_boundary(text, available.min(text.len()));
        let truncated = &text[..boundary];
        // Try to find a valid JSON boundary
        if let Some(pos) = truncated.rfind('}') {
            format!("{}{suffix}", &truncated[..=pos])
        } else {
            r#"{"truncated":true}"#.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fits_within_budget() {
        let budget = TokenBudget::from_tokens(100); // 400 bytes
        assert!(budget.fits("short text"));
        assert!(!budget.fits(&"x".repeat(500)));
    }

    #[test]
    fn test_truncate_json_fits() {
        let budget = TokenBudget::from_tokens(1000);
        let input = json!({"hits": [1, 2, 3]}).to_string();
        assert_eq!(budget.truncate_json(&input, "hits"), input);
    }

    #[test]
    fn test_truncate_json_removes_items() {
        let budget = TokenBudget::from_tokens(25); // 100 bytes
        let items: Vec<serde_json::Value> = (0..50)
            .map(|i| json!({"id": i, "name": format!("symbol_{i}")}))
            .collect();
        let input = json!({"q": "test", "hits": items}).to_string();
        let result = budget.truncate_json(&input, "hits");

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["total"], 50);
        assert!(result.len() <= 100);
    }

    #[test]
    fn test_truncate_json_empty_budget() {
        let budget = TokenBudget::from_tokens(1); // 4 bytes
        let input = json!({"hits": [1, 2, 3]}).to_string();
        let result = budget.truncate_json(&input, "hits");
        assert!(result.contains("truncated"));
    }
}
