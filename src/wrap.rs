//! Word wrapping for human help. Text is only ever broken at spaces, which cannot
//! fall inside a grapheme, so wrapping never needs to cut or mark a word.

use unicode_width::UnicodeWidthStr;

const TAB_STOP: usize = 8;

pub(super) fn cells(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Wrap free text such as descriptions, usage lines and examples. Each line keeps
/// its own leading indentation, and its continuation lines take the same
/// indentation. A width of zero returns text unchanged, which is what a
/// non-terminal stdout reports.
pub(super) fn wrap_prose(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_owned();
    }
    text.split('\n')
        .map(|line| {
            let expanded = expand_tabs(line);
            let (indent, body) = split_indent(expanded.trim_end_matches(' '));
            wrap_words(indent, body, indent, width).join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Break text between words so that lines fit `width` cells. The first line starts
/// with `first` and every later line with `cont`, both of which count toward the
/// width. Spaces inside a line survive, so an aligned example keeps its shape. A
/// word wider than the room left stays whole on its own line. A width of zero
/// returns everything on one line.
pub(super) fn wrap_words(first: &str, text: &str, cont: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![format!("{first}{text}")];
    }
    let mut lines = Vec::new();
    let mut line = first.to_owned();
    // A first line opened by a label can give up its first word, leaving the label
    // alone on its line. Bare indentation cannot: the word would only move down.
    let mut is_empty = first == cont || first.trim().is_empty();
    for (gap, word) in split_words(text) {
        if !is_empty && cells(&line) + cells(gap) + cells(word) > width {
            lines.push(std::mem::replace(&mut line, cont.to_owned()));
            is_empty = true;
        }
        if !is_empty {
            line.push_str(gap);
        }
        line.push_str(word);
        is_empty = false;
    }
    lines.push(line);
    lines
}

/// Split text at runs of spaces, keeping each run with the word after it.
/// `split_whitespace` would collapse them, flattening aligned examples.
fn split_words(mut text: &str) -> Vec<(&str, &str)> {
    let mut words = Vec::new();
    while !text.is_empty() {
        let rest = text.trim_start_matches(' ');
        let gap = &text[..text.len() - rest.len()];
        let end = rest.find(' ').unwrap_or(rest.len());
        words.push((gap, &rest[..end]));
        text = &rest[end..];
    }
    words
}

/// Separate a line's leading spaces from the rest of it.
pub(super) fn split_indent(line: &str) -> (&str, &str) {
    let body = line.trim_start_matches(' ');
    (&line[..line.len() - body.len()], body)
}

/// Remove spaces a caller's text left at line ends, where a reader cannot see them.
pub(super) fn trim_trailing_space(screen: &str) -> String {
    screen
        .split('\n')
        .map(|line| line.trim_end_matches(' '))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn expand_tabs(line: &str) -> String {
    let mut expanded = String::with_capacity(line.len());
    for character in line.chars() {
        if character == '\t' {
            let padding = TAB_STOP - cells(&expanded) % TAB_STOP;
            expanded.push_str(&" ".repeat(padding));
        } else {
            expanded.push(character);
        }
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::wrap_prose;

    #[test]
    fn wrap_prose_keeps_every_word_and_its_indentation() {
        for (text, width, want) in [
            ("\tkeeps   everything ", 0, "\tkeeps   everything "),
            ("    one two three", 12, "    one two\n    three"),
            ("a   b   c", 20, "a   b   c"),
            ("tiny enormousword tiny", 8, "tiny\nenormousword\ntiny"),
            ("界界 界界 界界", 10, "界界 界界\n界界"),
            ("one\n\ntwo", 10, "one\n\ntwo"),
            ("one   ", 10, "one"),
            ("a\tb\n界\tc", 20, "a       b\n界      c"),
            ("a\tb\tc", 20, "a       b       c"),
            ("12345678\tx", 20, "12345678        x"),
            ("\t\tx", 20, "                x"),
        ] {
            assert_eq!(wrap_prose(text, width), want, "{text:?} at {width}");
        }
    }
}
