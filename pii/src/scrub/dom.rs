use std::borrow::Cow;

use crate::context::Ctx;
use crate::schema::{
    AttrValue, FullSnapshotData, MutationData, SerializedNode, SerializedNodeWithId,
};

use super::{assets, text, url};

pub fn scrub_full_snapshot(ctx: &Ctx<'_>, snap: &mut FullSnapshotData<'_>) -> bool {
    walk_node(ctx, &mut snap.node, ParentKind::Other)
}

pub fn scrub_mutation(ctx: &Ctx<'_>, m: &mut MutationData<'_>) -> bool {
    let mut changed = false;
    let mut buf = String::new();
    for t in &mut m.texts {
        if let Some(v) = t.value.as_mut() {
            buf.clear();
            if text::scrub_into(ctx, v, &mut buf) {
                *v = Cow::Owned(std::mem::take(&mut buf));
                changed = true;
            }
        }
    }
    for a in &mut m.attributes {
        let kind = if a.attributes.keys().any(|k| assets::is_media_src_attr(k)) {
            TagKind::Media
        } else {
            TagKind::Other
        };
        changed |= scrub_attrs(ctx, &mut a.attributes, kind);
    }
    for added in &mut m.adds {
        changed |= walk_node(ctx, &mut added.node, ParentKind::Other);
    }
    changed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentKind {
    Script,
    Style,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Script,
    Style,
    Media,
    Other,
}

fn walk_node(ctx: &Ctx<'_>, node: &mut SerializedNodeWithId<'_>, parent: ParentKind) -> bool {
    let mut changed = false;
    match &mut node.node {
        SerializedNode::Element(el) => {
            let kind = classify_tag(&el.tag_name);
            changed |= scrub_attrs(ctx, &mut el.attributes, kind);
            let child_parent = match kind {
                TagKind::Script => ParentKind::Script,
                TagKind::Style => ParentKind::Style,
                _ => ParentKind::Other,
            };
            for child in &mut el.child_nodes {
                changed |= walk_node(ctx, child, child_parent);
            }
        }
        SerializedNode::Document(d) => {
            for child in &mut d.child_nodes {
                changed |= walk_node(ctx, child, ParentKind::Other);
            }
        }
        SerializedNode::Text(t) => {
            if parent == ParentKind::Script
                || parent == ParentKind::Style
                || t.is_style == Some(true)
            {
                return false;
            }
            let mut buf = String::new();
            if text::scrub_into(ctx, &t.text_content, &mut buf) {
                t.text_content = Cow::Owned(buf);
                changed = true;
            }
        }
        SerializedNode::Comment(c) => {
            let mut buf = String::new();
            if text::scrub_into(ctx, &c.text_content, &mut buf) {
                c.text_content = Cow::Owned(buf);
                changed = true;
            }
        }
        SerializedNode::DocumentType(_) | SerializedNode::Cdata(_) => {}
    }
    changed
}

fn classify_tag(tag: &str) -> TagKind {
    if tag.eq_ignore_ascii_case("script") {
        TagKind::Script
    } else if tag.eq_ignore_ascii_case("style") {
        TagKind::Style
    } else if assets::is_media_tag(tag) {
        TagKind::Media
    } else {
        TagKind::Other
    }
}

fn scrub_attrs<'a>(
    ctx: &Ctx<'_>,
    attrs: &mut rustc_hash::FxHashMap<Cow<'a, str>, AttrValue<'a>>,
    kind: TagKind,
) -> bool {
    let mut changed = false;
    let mut buf = String::new();
    for (name, val) in attrs.iter_mut() {
        if kind == TagKind::Media && assets::is_media_src_attr(name) {
            continue;
        }
        let Some(s) = attr_str(val) else { continue };
        let was = if is_url_attr(name) {
            buf.clear();
            url::scrub_into(ctx, s, &mut buf)
        } else if is_user_text_attr(name) {
            buf.clear();
            text::scrub_into(ctx, s, &mut buf)
        } else {
            continue;
        };
        if was {
            *val = AttrValue::Str(Cow::Owned(std::mem::take(&mut buf)));
            changed = true;
        }
    }

    if kind == TagKind::Media {
        assets::apply_blur(ctx, attrs);
        changed = true;
    }
    changed
}

fn attr_str<'a>(v: &'a AttrValue<'_>) -> Option<&'a str> {
    match v {
        AttrValue::Str(s) => Some(s.as_ref()),
        _ => None,
    }
}

fn is_user_text_attr(name: &str) -> bool {
    matches!(
        name,
        "alt"
            | "title"
            | "placeholder"
            | "aria-label"
            | "aria-description"
            | "aria-roledescription"
            | "aria-valuetext"
            | "aria-placeholder"
            | "value"
            | "label"
            | "summary"
    )
}

fn is_url_attr(name: &str) -> bool {
    matches!(
        name,
        "href"
            | "src"
            | "srcset"
            | "action"
            | "formaction"
            | "cite"
            | "data"
            | "poster"
            | "background"
            | "xlink:href"
            | "manifest"
            | "longdesc"
    )
}
