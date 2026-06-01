use simd_json::OwnedValue;

use crate::context::Ctx;

use super::{text, url};

pub fn scrub_value_generic(ctx: &Ctx<'_>, v: &mut OwnedValue) -> bool {
    let mut changed = false;
    let mut buf = String::new();
    walk(ctx, v, &mut buf, &mut changed);
    changed
}

fn walk(ctx: &Ctx<'_>, v: &mut OwnedValue, buf: &mut String, changed: &mut bool) {
    match v {
        OwnedValue::String(s) => {
            buf.clear();
            let was = if looks_like_url(s) {
                url::scrub_into(ctx, s, buf)
            } else {
                text::scrub_into(ctx, s, buf)
            };
            if was {
                *s = std::mem::take(buf);
                *changed = true;
            }
        }
        OwnedValue::Array(arr) => {
            for item in arr.iter_mut() {
                walk(ctx, item, buf, changed);
            }
        }
        OwnedValue::Object(obj) => {
            for (_, val) in obj.iter_mut() {
                walk(ctx, val, buf, changed);
            }
        }
        _ => {}
    }
}

/// rrweb/network@1 payload: `{ requests: CapturedNetworkRequest[] }`. Per
/// request: `name` is the Resource Timing URL (URL-scrub); request/response
/// bodies + every header value are free text.
pub fn scrub_network_plugin(ctx: &Ctx<'_>, payload: &mut OwnedValue) -> bool {
    let mut changed = false;
    let mut buf = String::new();
    let OwnedValue::Object(obj) = payload else {
        return scrub_value_generic(ctx, payload);
    };
    let Some(OwnedValue::Array(reqs)) = obj.get_mut("requests") else {
        return false;
    };
    for req in reqs.iter_mut() {
        let OwnedValue::Object(req_obj) = req else { continue };
        if let Some(OwnedValue::String(name)) = req_obj.get_mut("name") {
            buf.clear();
            if url::scrub_into(ctx, name, &mut buf) {
                *name = std::mem::take(&mut buf);
                changed = true;
            }
        }
        for field in ["requestBody", "responseBody"] {
            if let Some(OwnedValue::String(s)) = req_obj.get_mut(field) {
                buf.clear();
                if text::scrub_into(ctx, s, &mut buf) {
                    *s = std::mem::take(&mut buf);
                    changed = true;
                }
            }
        }
        for field in ["requestHeaders", "responseHeaders"] {
            if let Some(OwnedValue::Object(hdrs)) = req_obj.get_mut(field) {
                for (_, v) in hdrs.iter_mut() {
                    if let OwnedValue::String(s) = v {
                        buf.clear();
                        if text::scrub_into(ctx, s, &mut buf) {
                            *s = std::mem::take(&mut buf);
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    changed
}

/// rrweb/console@1 payload: `{ level, payload: string[], trace: string[] }`.
pub fn scrub_console_plugin(ctx: &Ctx<'_>, payload: &mut OwnedValue) -> bool {
    let mut changed = false;
    let mut buf = String::new();
    let OwnedValue::Object(obj) = payload else {
        return scrub_value_generic(ctx, payload);
    };
    for field in ["payload", "trace"] {
        if let Some(OwnedValue::Array(arr)) = obj.get_mut(field) {
            for v in arr.iter_mut() {
                if let OwnedValue::String(s) = v {
                    buf.clear();
                    if text::scrub_into(ctx, s, &mut buf) {
                        *s = std::mem::take(&mut buf);
                        changed = true;
                    }
                }
            }
        }
    }
    changed
}

fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}
