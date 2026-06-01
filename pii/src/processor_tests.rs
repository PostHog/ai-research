use super::*;
use crate::config::Config;
use crate::dict::AllowLists;

fn run_line(line: &[u8]) -> Vec<u8> {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let ctx = Ctx::new(&cfg, &allow);
    let mut out = Vec::new();
    let mut scratch = Scratch::default();
    process_line(&ctx, line, &mut scratch, &mut out);
    out
}

#[test]
fn passthrough_mousemove() {
    let line = br#"{"type":3,"timestamp":1,"data":{"source":1,"positions":[{"x":1.0,"y":2.0,"id":3,"timeOffset":4}]}}"#;
    assert_eq!(run_line(line), line);
}

#[test]
fn full_snapshot_roundtrip_through_scrub_stub() {
    let line = br#"{"type":2,"timestamp":1,"data":{"node":{"type":0,"id":1,"childNodes":[]},"initialOffset":{"top":0.0,"left":0.0}}}"#;
    let out = run_line(line);
    use crate::schema::{FullSnapshotData, data_value_range};
    let (s_in, e_in) = data_value_range(line).unwrap();
    let (s_out, e_out) = data_value_range(&out).unwrap();
    let mut buf_in = line[s_in..e_in].to_vec();
    let mut buf_out = out[s_out..e_out].to_vec();
    let _: FullSnapshotData = simd_json::serde::from_slice(&mut buf_in).unwrap();
    let _: FullSnapshotData = simd_json::serde::from_slice(&mut buf_out).unwrap();
    assert_eq!(&line[..s_in], &out[..s_out]);
    assert_eq!(&line[e_in..], &out[e_out..]);
}

#[test]
fn meta_url_scrubbed_through_processor() {
    let line = br#"{"type":4,"timestamp":1,"data":{"href":"https://example.com/api/v1/users/abc","width":1280,"height":720}}"#;
    let out = run_line(line);
    let s = std::str::from_utf8(&out).unwrap();
    assert!(
        s.contains(r#""href":"https://example.com/api/v1/users/[redacted]""#),
        "got: {s}"
    );
    assert!(s.contains(r#""width":1280"#), "got: {s}");
    assert!(s.contains(r#""height":720"#), "got: {s}");
}
