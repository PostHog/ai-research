use crate::context::Ctx;

#[allow(dead_code)]
pub fn scrub(ctx: &Ctx<'_>, input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    scrub_into(ctx, input, &mut out);
    out
}

pub fn scrub_into(ctx: &Ctx<'_>, input: &str, out: &mut String) -> bool {
    let allow = ctx.allow;
    let (path_and_authority, dropped) = split_at_any(input, &['?', '#']).unwrap_or((input, ""));
    let mut changed = !dropped.is_empty();
    let (prefix, path) = split_authority(path_and_authority);
    out.push_str(prefix);

    let mut first = true;
    for raw in path.split('/') {
        if first {
            first = false;
        } else {
            out.push('/');
        }
        if raw.is_empty() {
            continue;
        }
        if is_safe_segment(raw) || allow.url_contains(raw) {
            out.push_str(raw);
        } else {
            out.push_str("[redacted]");
            changed = true;
        }
    }
    changed
}

fn split_authority(s: &str) -> (&str, &str) {
    if let Some(scheme_end) = s.find("://") {
        let after = &s[scheme_end + 3..];
        if let Some(path_off) = after.find('/') {
            let split = scheme_end + 3 + path_off;
            return (&s[..split], &s[split..]);
        }
        return (s, "");
    }
    if let Some(rest) = s.strip_prefix("//") {
        if let Some(path_off) = rest.find('/') {
            let split = 2 + path_off;
            return (&s[..split], &s[split..]);
        }
        return (s, "");
    }
    ("", s)
}

fn split_at_any<'a>(s: &'a str, delims: &[char]) -> Option<(&'a str, &'a str)> {
    let idx = s.find(|c| delims.contains(&c))?;
    Some((&s[..idx], &s[idx..]))
}

fn is_safe_segment(seg: &str) -> bool {
    matches!(seg, "" | "." | "..")
}
