use crate::config::Config;
use crate::context::Ctx;
use crate::dict::AllowLists;

#[allow(dead_code)]
pub fn scrub(ctx: &Ctx<'_>, input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    scrub_into(ctx, input, &mut out);
    out
}

pub fn scrub_into(ctx: &Ctx<'_>, input: &str, out: &mut String) -> bool {
    let force_redact_all = count_words(input) > ctx.config.max_words_len;
    let allow = ctx.allow;
    let mut changed = false;
    let mut chars = input.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        if is_word_char(c) {
            let mut end = start;
            while let Some(&(i, ch)) = chars.peek() {
                if is_word_char(ch) {
                    end = i + ch.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            emit_word(
                &input[start..end],
                allow,
                force_redact_all,
                out,
                &mut changed,
            );
        } else {
            out.push(c);
            chars.next();
        }
    }
    changed
}

fn count_words(input: &str) -> usize {
    let mut n = 0;
    let mut in_word = false;
    for c in input.chars() {
        if is_word_char(c) {
            if !in_word {
                n += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    n
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\'' || c == '\u{2019}'
}

fn emit_word(
    word: &str,
    allow: &AllowLists,
    force_redact_all: bool,
    out: &mut String,
    changed: &mut bool,
) {
    if is_numeric_token(word) {
        push_redacted(word, Config::NUMBER_CHAR, out);
        *changed = true;
        return;
    }
    if force_redact_all {
        push_redacted(word, Config::REDACT_CHAR, out);
        *changed = true;
        return;
    }
    if word_is_allowed(allow, word) {
        out.push_str(word);
    } else {
        push_redacted(word, Config::REDACT_CHAR, out);
        *changed = true;
    }
}

fn push_redacted(word: &str, mark: char, out: &mut String) {
    for _ in word.chars() {
        out.push(mark);
    }
}

fn word_is_allowed(allow: &AllowLists, word: &str) -> bool {
    if allow.text_contains(word) {
        return true;
    }
    if word.contains('\u{2019}') {
        let normalized = word.replace('\u{2019}', "'");
        if allow.text_contains(&normalized) {
            return true;
        }
        if let Some(b) = strip_possessive(&normalized) {
            if allow.text_contains(b) {
                return true;
            }
        }
    }
    if let Some(b) = strip_possessive(word) {
        if allow.text_contains(b) {
            return true;
        }
    }
    false
}

fn strip_possessive(word: &str) -> Option<&str> {
    for suffix in ["'s", "\u{2019}s", "'", "\u{2019}"] {
        if let Some(b) = word.strip_suffix(suffix)
            && !b.is_empty()
        {
            return Some(b);
        }
    }
    None
}

fn is_numeric_token(word: &str) -> bool {
    let mut saw_digit = false;
    for c in word.chars() {
        if c.is_ascii_digit() {
            saw_digit = true;
        } else if !matches!(c, '.' | ',' | '-' | '+') {
            return false;
        }
    }
    saw_digit
}
