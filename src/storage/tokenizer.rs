/// Splits code identifiers into searchable subwords.
///
/// Handles camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE,
/// and acronym boundaries (e.g., HTMLParser → html parser).
/// Returns lowercased space-separated subwords plus the original name lowercased.
///
/// Example: `"getUserById"` → `"get user by id getuserbyid"`
pub fn split_code_identifier(name: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = name.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];

        if c == '_' || c == '-' || c == '.' || c == ' ' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }

        if c.is_uppercase() {
            if !current.is_empty() {
                let prev_lower = i > 0 && chars[i - 1].is_lowercase();
                let next_lower = i + 1 < len && chars[i + 1].is_lowercase();

                if prev_lower || next_lower {
                    // camelCase: "getUser" → split before U
                    // Acronym end: "HTMLParser" → split before P (keeps "html" intact)
                    words.push(std::mem::take(&mut current));
                }
            }
            current.push(c.to_ascii_lowercase());
        } else {
            current.push(c.to_ascii_lowercase());
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    if words.is_empty() {
        return name.to_lowercase();
    }

    let lowered = name.to_lowercase();
    let joined = words.join(" ");

    // Append original lowercased name if different from joined words
    if joined == lowered {
        joined
    } else {
        format!("{joined} {lowered}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case() {
        assert_eq!(
            split_code_identifier("getUserById"),
            "get user by id getuserbyid"
        );
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(
            split_code_identifier("UserRepository"),
            "user repository userrepository"
        );
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(
            split_code_identifier("user_repository"),
            "user repository user_repository"
        );
    }

    #[test]
    fn test_screaming_snake() {
        assert_eq!(
            split_code_identifier("MAX_RETRIES"),
            "max retries max_retries"
        );
    }

    #[test]
    fn test_acronym_boundary() {
        assert_eq!(
            split_code_identifier("HTMLParser"),
            "html parser htmlparser"
        );
    }

    #[test]
    fn test_simple_word() {
        assert_eq!(split_code_identifier("add"), "add");
    }

    #[test]
    fn test_single_char() {
        assert_eq!(split_code_identifier("x"), "x");
    }

    #[test]
    fn test_all_caps() {
        assert_eq!(split_code_identifier("HTTP"), "http");
    }

    #[test]
    fn test_mixed_separators() {
        assert_eq!(
            split_code_identifier("get_UserName"),
            "get user name get_username"
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(split_code_identifier(""), "");
    }
}
