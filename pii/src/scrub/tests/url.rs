use crate::config::Config;
use crate::context::Ctx;
use crate::dict::AllowLists;
use crate::scrub::url::scrub;

fn ctx<'a>(cfg: &'a Config, allow: &'a AllowLists) -> Ctx<'a> {
    Ctx::new(cfg, allow)
}

#[test]
fn keeps_allowed_segments() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(
        &ctx(&cfg, &allow),
        "https://example.com/api/v1/users/42/profile",
    );
    assert_eq!(out, "https://example.com/api/v1/users/[redacted]/profile");
}

#[test]
fn drops_query_and_fragment() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(
        &ctx(&cfg, &allow),
        "https://example.com/dashboard?token=secret#frag",
    );
    assert_eq!(out, "https://example.com/dashboard");
}

#[test]
fn relative_path() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "/user/abc/edit");
    assert_eq!(out, "/user/[redacted]/edit");
}
