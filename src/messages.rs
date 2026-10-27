use std::path::Path;

/// Fold home paths in diagnostics without changing filesystem paths or sibling names.
pub fn display_message(text: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return text.into();
    };
    fold_message(text, &home)
}

/// Fold only whole home-path segments, including quoted paths in error messages.
pub fn fold_message(text: &str, home: &Path) -> String {
    let home = home.to_string_lossy();
    if home.is_empty() {
        return text.into();
    }
    let mut result = String::new();
    let mut position = 0;
    for (start, _) in text.match_indices(home.as_ref()) {
        let end = start + home.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if before
            .is_none_or(|c| c.is_whitespace() || matches!(c, '\'' | '"' | '(' | '[' | ':' | '='))
            && after.is_none_or(|c| {
                c == std::path::MAIN_SEPARATOR
                    || c.is_whitespace()
                    || matches!(c, '\'' | '"' | ')' | ']' | ';' | ':' | ',')
            })
        {
            result.push_str(&text[position..start]);
            result.push('~');
            position = end;
        }
    }
    result.push_str(&text[position..]);
    result
}
