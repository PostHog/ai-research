use std::borrow::Cow;

use rustc_hash::FxHashMap;

use crate::config::Config;
use crate::context::Ctx;
use crate::dict::AllowLists;
use crate::schema::AttrValue;
use crate::scrub::assets::{apply_blur, is_media_tag, PLACEHOLDER_SRC};

#[test]
fn media_tags_are_classified() {
    assert!(is_media_tag("img"));
    assert!(is_media_tag("IMG"));
    assert!(is_media_tag("image"));
    assert!(is_media_tag("video"));
    assert!(is_media_tag("audio"));
    assert!(is_media_tag("source"));
    assert!(is_media_tag("track"));
    assert!(!is_media_tag("iframe"));
    assert!(!is_media_tag("div"));
    assert!(!is_media_tag("a"));
}

#[test]
fn remote_src_is_replaced_with_placeholder_and_preserved() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let ctx = Ctx::new(&cfg, &allow);
    let mut attrs: FxHashMap<Cow<'_, str>, AttrValue<'_>> = FxHashMap::default();
    attrs.insert(
        Cow::Borrowed("src"),
        AttrValue::Str(Cow::Borrowed("https://example.com/u/abc.png")),
    );
    apply_blur(&ctx, &mut attrs);
    match attrs.get(&Cow::Borrowed("src")).unwrap() {
        AttrValue::Str(s) => assert_eq!(s, PLACEHOLDER_SRC),
        _ => panic!(),
    }
    assert!(attrs.contains_key(&Cow::Borrowed("data-original-src")));
}
