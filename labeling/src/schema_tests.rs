use std::borrow::Cow;

use super::*;

fn parse<'a, T: serde::Deserialize<'a>>(buf: &'a mut [u8]) -> T {
    simd_json::serde::from_slice(buf).expect("simd-json parse")
}

#[test]
fn meta_roundtrip() {
    let line = br#"{"type":4,"timestamp":1700000000000,"data":{"href":"https://app","width":1280,"height":720}}"#;
    let mut buf = line.to_vec();
    let evt: Event<MetaData> = parse(&mut buf);
    assert_eq!(evt.ty, 4);
    assert_eq!(evt.data.href, "https://app");
    assert_eq!(evt.data.width, 1280);
    let out = simd_json::serde::to_string(&evt).unwrap();
    assert!(out.contains("\"type\":4"), "got: {out}");
    assert!(out.contains("\"href\":\"https://app\""), "got: {out}");
}

#[test]
fn full_snapshot_with_dom() {
    let line = br#"{"type":2,"timestamp":1,"data":{"node":{"type":0,"id":1,"childNodes":[{"type":2,"id":2,"tagName":"html","attributes":{"lang":"en"},"childNodes":[{"type":2,"id":3,"tagName":"body","attributes":{"class":"x","data-n":3,"disabled":true,"removed":null},"childNodes":[{"type":3,"id":4,"textContent":"hi"}]}]}]},"initialOffset":{"top":0,"left":0}}}"#;
    let mut buf = line.to_vec();
    let evt: Event<FullSnapshotData> = parse(&mut buf);
    let root = match &evt.data.node.node {
        SerializedNode::Document(d) => d,
        _ => panic!("expected Document at root"),
    };
    assert_eq!(root.child_nodes.len(), 1);
    let html = match &root.child_nodes[0].node {
        SerializedNode::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(html.tag_name, "html");
    let body = match &html.child_nodes[0].node {
        SerializedNode::Element(e) => e,
        _ => panic!(),
    };
    assert_eq!(body.tag_name, "body");
    assert!(matches!(body.attributes.get("class").unwrap(), AttrValue::Str(s) if s == "x"));
    assert!(matches!(body.attributes.get("data-n").unwrap(), AttrValue::Num(n) if *n == 3.0));
    assert!(matches!(
        body.attributes.get("disabled").unwrap(),
        AttrValue::Bool(true)
    ));
    assert!(matches!(
        body.attributes.get("removed").unwrap(),
        AttrValue::Null
    ));
    let text = match &body.child_nodes[0].node {
        SerializedNode::Text(t) => t,
        _ => panic!(),
    };
    assert_eq!(text.text_content, "hi");
}

#[test]
fn mutation_inline() {
    let line = br#"{"type":3,"timestamp":2,"data":{"source":0,"texts":[{"id":5,"value":"hello"}],"attributes":[{"id":4,"attributes":{"class":"y"}}],"removes":[{"parentId":3,"id":7}],"adds":[{"parentId":4,"nextId":null,"node":{"type":3,"id":8,"textContent":"!"}}]}}"#;
    let mut buf = line.to_vec();
    let evt: Event<MutationData> = parse(&mut buf);
    assert_eq!(evt.data.source, IncrementalSource::Mutation);
    assert_eq!(evt.data.texts[0].value.as_deref(), Some("hello"));
    assert_eq!(evt.data.attributes[0].id, 4);
    assert_eq!(evt.data.removes[0].id, 7);
    assert_eq!(evt.data.adds[0].parent_id, 4);
}

#[test]
fn input_data() {
    let line = br#"{"type":3,"timestamp":3,"data":{"source":5,"id":1,"text":"hello","isChecked":false}}"#;
    let mut buf = line.to_vec();
    let evt: Event<InputData> = parse(&mut buf);
    assert_eq!(evt.data.text, "hello");
    assert!(!evt.data.is_checked);
}

#[test]
fn windowed_tuple_roundtrip() {
    let line = br#"["w1",{"type":4,"timestamp":1,"data":{"href":"/","width":1,"height":1}}]"#;
    let mut buf = line.to_vec();
    let WindowedEvent { window_id, event }: WindowedEvent<Event<MetaData>> = parse(&mut buf);
    assert_eq!(window_id, "w1");
    assert_eq!(event.data.width, 1);

    let back = WindowedEvent { window_id, event };
    let out = simd_json::serde::to_string(&back).unwrap();
    assert!(out.starts_with('['), "got: {out}");
    assert!(out.contains("\"w1\""), "got: {out}");
}

#[test]
fn full_snapshot_compressed_envelope() {
    // Just the envelope round-trip — gunzip-and-reparse of the inner
    // payload is the caller's job. `data` is a quoted string.
    let line = br#"{"type":2,"timestamp":1,"cv":"2024-10","data":"H4sIAAAAAAAA"}"#;
    let mut buf = line.to_vec();
    let evt: EventCompressed<Cow<str>> = parse(&mut buf);
    assert_eq!(evt.cv, "2024-10");
    assert_eq!(evt.data, "H4sIAAAAAAAA");
}

#[test]
fn mutation_compressed_subfields() {
    // Compressed Mutation: top-level data is an object, but each of
    // texts/attributes/removes/adds is a string (gzip-base64).
    let line = br#"{"type":3,"timestamp":1,"cv":"2024-10","data":{"source":0,"texts":"AAA","attributes":"BBB","removes":"CCC","adds":"DDD"}}"#;
    let mut buf = line.to_vec();
    let evt: EventCompressed<MutationDataCompressed> = parse(&mut buf);
    assert_eq!(evt.data.source, IncrementalSource::Mutation);
    assert_eq!(evt.data.texts, "AAA");
    assert_eq!(evt.data.adds, "DDD");
}

#[test]
fn empty_data_event() {
    // type=0 / type=1: data is {}
    let line = br#"{"type":1,"timestamp":1,"data":{}}"#;
    let mut buf = line.to_vec();
    let evt: Event<EmptyData> = parse(&mut buf);
    assert_eq!(evt.ty, 1);
    let _ = evt.data;
}

#[test]
fn data_range_finds_object() {
    let line = br#"{"type":4,"timestamp":1,"data":{"href":"/x","width":1,"height":1}}"#;
    let (s, e) = data_value_range(line).unwrap();
    assert_eq!(&line[s..e], &br#"{"href":"/x","width":1,"height":1}"#[..]);
}

#[test]
fn data_range_finds_string() {
    let line = br#"{"type":2,"timestamp":1,"cv":"2024-10","data":"abc\"def"}"#;
    let (s, e) = data_value_range(line).unwrap();
    assert_eq!(&line[s..e], &br#""abc\"def""#[..]);
}

#[test]
fn data_range_ignores_nested_data_key() {
    let line = br#"{"type":5,"timestamp":1,"data":{"tag":"x","payload":{"data":42}}}"#;
    let (s, e) = data_value_range(line).unwrap();
    assert_eq!(&line[s..e], &br#"{"tag":"x","payload":{"data":42}}"#[..]);
}

#[test]
fn data_range_finds_data_in_windowed_tuple() {
    let line =
        br#"["w1",{"type":4,"timestamp":1,"data":{"href":"/x","width":1,"height":1}}]"#;
    let (s, e) = data_value_range(line).unwrap();
    assert_eq!(&line[s..e], &br#"{"href":"/x","width":1,"height":1}"#[..]);
}

#[test]
fn extract_payload_uncompressed_copies_bytes() {
    let line = br#"{"type":4,"timestamp":1,"data":{"href":"/x","width":1,"height":1}}"#;
    let mut scratch = Vec::new();
    let (s, e) = extract_payload(line, false, &mut scratch).unwrap();
    assert_eq!(scratch, &line[s..e]);
}

#[test]
fn extract_payload_compressed_roundtrip() {
    let payload = br#"{"node":{"type":0,"id":1,"childNodes":[]},"initialOffset":{"top":0,"left":0}}"#;
    let mut env_line = Vec::new();
    env_line.extend_from_slice(br#"{"type":2,"timestamp":1,"cv":"2024-10","data":"#);
    write_compressed_string(payload, &mut env_line).unwrap();
    env_line.push(b'}');

    let mut scratch = Vec::new();
    let _ = extract_payload(&env_line, true, &mut scratch).unwrap();
    assert_eq!(scratch, payload, "decompressed payload should match original");
}

#[test]
fn emit_with_payload_uncompressed_splices() {
    let line = br#"{"type":4,"timestamp":1,"data":{"href":"/x","width":1,"height":1}}"#;
    let new_payload = br#"{"href":"/[redacted]","width":1,"height":1}"#;
    let (s, e) = data_value_range(line).unwrap();
    let mut out = Vec::new();
    emit_with_payload(line, (s, e), new_payload, false, &mut out).unwrap();
    let expected = br#"{"type":4,"timestamp":1,"data":{"href":"/[redacted]","width":1,"height":1}}"#;
    assert_eq!(&out[..], &expected[..]);
}

#[test]
fn emit_with_payload_compressed_roundtrip() {
    let line = br#"{"type":2,"timestamp":1,"cv":"2024-10","data":"placeholder"}"#;
    let new_payload =
        br#"{"node":{"type":0,"id":1,"childNodes":[]},"initialOffset":{"top":0,"left":0}}"#;
    let (s, e) = data_value_range(line).unwrap();
    let mut out = Vec::new();
    emit_with_payload(line, (s, e), new_payload, true, &mut out).unwrap();

    let mut scratch = Vec::new();
    let _ = extract_payload(&out, true, &mut scratch).unwrap();
    assert_eq!(scratch, new_payload);
}

#[test]
fn full_snapshot_compressed_read_write_roundtrip() {
    // Build a compressed FullSnapshot line, read+write it back, verify the
    // inner payload survives.
    let payload =
        br#"{"node":{"type":0,"id":1,"childNodes":[]},"initialOffset":{"top":0.0,"left":0.0}}"#;
    let mut line = Vec::new();
    line.extend_from_slice(br#"{"type":2,"timestamp":1,"cv":"2024-10","data":"#);
    write_compressed_string(payload, &mut line).unwrap();
    line.push(b'}');

    let mut data_buf = Vec::new();
    let mut payload_buf = Vec::new();
    let mut out = Vec::new();
    let (range, data) = FullSnapshotData::read(&line, true, &mut data_buf).unwrap();
    data.write(&line, range, true, &mut payload_buf, &mut out).unwrap();

    // Decompress the output and verify it parses back to the same shape.
    let mut roundtrip_buf = Vec::new();
    let _ = extract_payload(&out, true, &mut roundtrip_buf).unwrap();
    let roundtrip: FullSnapshotData = simd_json::serde::from_slice(&mut roundtrip_buf).unwrap();
    let _ = roundtrip; // structural roundtrip is what we're after
}

#[test]
fn mutation_uncompressed_read_write_roundtrip() {
    let line = br#"{"type":3,"timestamp":1,"data":{"source":0,"texts":[{"id":5,"value":"hello"}],"attributes":[{"id":4,"attributes":{"class":"y"}}],"removes":[{"parentId":3,"id":7}],"adds":[{"parentId":4,"nextId":null,"node":{"type":3,"id":8,"textContent":"!"}}]}}"#;
    let mut data_buf = Vec::new();
    let mut sub = MutationSubScratch::default();
    let mut payload_buf = Vec::new();
    let mut out = Vec::new();
    let (range, data) = MutationData::read(line, false, &mut data_buf, &mut sub).unwrap();
    assert_eq!(data.texts[0].value.as_deref(), Some("hello"));
    data.write(line, range, false, &mut payload_buf, &mut out).unwrap();

    // Envelope preserved
    assert!(out.starts_with(br#"{"type":3,"timestamp":1,"data":"#));
    assert!(out.ends_with(b"}"));
    let s = std::str::from_utf8(&out).unwrap();
    assert!(s.contains(r#""value":"hello""#), "got: {s}");
}

#[test]
fn mutation_compressed_read_write_roundtrip() {
    // Build a compressed Mutation line by gzipping each sub-field array.
    let texts_arr = br#"[{"id":5,"value":"hi"}]"#;
    let attributes_arr = br#"[]"#;
    let removes_arr = br#"[]"#;
    let adds_arr = br#"[]"#;

    let texts_s = compress_subfield_to_string(texts_arr).unwrap();
    let attributes_s = compress_subfield_to_string(attributes_arr).unwrap();
    let removes_s = compress_subfield_to_string(removes_arr).unwrap();
    let adds_s = compress_subfield_to_string(adds_arr).unwrap();

    let cd = MutationDataCompressed {
        source: IncrementalSource::Mutation,
        texts: Cow::Owned(texts_s),
        attributes: Cow::Owned(attributes_s),
        removes: Cow::Owned(removes_s),
        adds: Cow::Owned(adds_s),
        is_attach_iframe: None,
    };
    let evt = EventCompressed {
        ty: 3,
        timestamp: 1,
        delay: None,
        cv: Cow::Borrowed("2024-10"),
        data: cd,
    };
    let line = simd_json::serde::to_vec(&evt).unwrap();

    // Read + write the compressed line.
    let mut data_buf = Vec::new();
    let mut sub = MutationSubScratch::default();
    let mut payload_buf = Vec::new();
    let mut out = Vec::new();
    let (range, data) = MutationData::read(&line, true, &mut data_buf, &mut sub).unwrap();
    assert_eq!(data.source, IncrementalSource::Mutation);
    assert_eq!(data.texts.len(), 1);
    assert_eq!(data.texts[0].id, 5);
    assert_eq!(data.texts[0].value.as_deref(), Some("hi"));
    data.write(&line, range, true, &mut payload_buf, &mut out).unwrap();

    // Roundtrip: read the freshly written line, verify same structure.
    let mut data_buf2 = Vec::new();
    let mut sub2 = MutationSubScratch::default();
    let (_, data2) = MutationData::read(&out, true, &mut data_buf2, &mut sub2).unwrap();
    assert_eq!(data2.texts.len(), 1);
    assert_eq!(data2.texts[0].value.as_deref(), Some("hi"));
}

