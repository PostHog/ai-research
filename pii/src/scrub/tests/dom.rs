use crate::config::Config;
use crate::context::Ctx;
use crate::dict::AllowLists;
use crate::schema::{AttrValue, Event, FullSnapshotData, SerializedNode};
use crate::scrub::dom::scrub_full_snapshot;

fn parse_full_snapshot(line: &str) -> FullSnapshotData<'static> {
    let buf: &'static mut [u8] = Box::leak(line.as_bytes().to_vec().into_boxed_slice());
    let evt: Event<FullSnapshotData<'static>> = simd_json::serde::from_slice(buf).unwrap();
    evt.data
}

#[test]
fn scrubs_text_node_but_keeps_script() {
    let line = r#"{"type":2,"timestamp":1,"data":{"node":{"type":0,"id":1,"childNodes":[{"type":2,"id":2,"tagName":"div","attributes":{},"childNodes":[{"type":3,"id":3,"textContent":"Hello SecretName"}]},{"type":2,"id":4,"tagName":"script","attributes":{},"childNodes":[{"type":3,"id":5,"textContent":"var x = 1;"}]}]},"initialOffset":{"top":0,"left":0}}}"#;
    let mut snap = parse_full_snapshot(line);
    let cfg = Config::default();
    let allow = AllowLists::default();
    scrub_full_snapshot(&Ctx::new(&cfg, &allow), &mut snap);

    let root = match snap.node.node {
        SerializedNode::Document(d) => d,
        _ => unreachable!(),
    };
    let div = match &root.child_nodes[0].node {
        SerializedNode::Element(e) => e,
        _ => unreachable!(),
    };
    let txt = match &div.child_nodes[0].node {
        SerializedNode::Text(t) => t,
        _ => unreachable!(),
    };
    // "Hello" is allow-listed; "SecretName" is 10 chars → ten *s.
    assert_eq!(txt.text_content, "Hello **********");

    // Script subtree text is left untouched (it's source code).
    let script = match &root.child_nodes[1].node {
        SerializedNode::Element(e) => e,
        _ => unreachable!(),
    };
    let script_txt = match &script.child_nodes[0].node {
        SerializedNode::Text(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(script_txt.text_content, "var x = 1;");
}

#[test]
fn url_attrs_get_url_scrub() {
    let line = r#"{"type":2,"timestamp":1,"data":{"node":{"type":0,"id":1,"childNodes":[{"type":2,"id":2,"tagName":"a","attributes":{"href":"https://example.com/user/abc/edit","class":"link primary"},"childNodes":[]}]},"initialOffset":{"top":0,"left":0}}}"#;
    let mut snap = parse_full_snapshot(line);
    let cfg = Config::default();
    let allow = AllowLists::default();
    scrub_full_snapshot(&Ctx::new(&cfg, &allow), &mut snap);

    let root = match snap.node.node {
        SerializedNode::Document(d) => d,
        _ => unreachable!(),
    };
    let a = match &root.child_nodes[0].node {
        SerializedNode::Element(e) => e,
        _ => unreachable!(),
    };
    match a.attributes.get("href").unwrap() {
        AttrValue::Str(s) => assert_eq!(s, "https://example.com/user/[redacted]/edit"),
        _ => panic!("expected str"),
    }
    // `class` is not a URL or user-text attr → untouched.
    match a.attributes.get("class").unwrap() {
        AttrValue::Str(s) => assert_eq!(s, "link primary"),
        _ => panic!("expected str"),
    }
}

#[test]
fn image_remote_src_becomes_placeholder_and_preserves_url() {
    let line = r#"{"type":2,"timestamp":1,"data":{"node":{"type":0,"id":1,"childNodes":[{"type":2,"id":2,"tagName":"img","attributes":{"src":"https://example.com/u/abc.png","alt":"profile photo of user"},"childNodes":[]}]},"initialOffset":{"top":0,"left":0}}}"#;
    let mut snap = parse_full_snapshot(line);
    let cfg = Config::default();
    let allow = AllowLists::default();
    scrub_full_snapshot(&Ctx::new(&cfg, &allow), &mut snap);

    let root = match snap.node.node {
        SerializedNode::Document(d) => d,
        _ => unreachable!(),
    };
    let img = match &root.child_nodes[0].node {
        SerializedNode::Element(e) => e,
        _ => unreachable!(),
    };
    match img.attributes.get("src").unwrap() {
        AttrValue::Str(s) => assert!(s.starts_with("data:image/svg+xml")),
        _ => panic!(),
    }
    assert!(img.attributes.contains_key("data-original-src"));
}
