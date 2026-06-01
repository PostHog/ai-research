use crate::config::Config;
use crate::context::Ctx;
use crate::dict::AllowLists;
use crate::scrub::text::scrub;

fn ctx<'a>(cfg: &'a Config, allow: &'a AllowLists) -> Ctx<'a> {
    Ctx::new(cfg, allow)
}

#[test]
fn allowlisted_words_kept() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "Click submit to continue");
    assert_eq!(out, "Click submit to continue");
}

#[test]
fn unknown_words_redacted_per_char() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "Hello Mr Smithson");
    assert_eq!(out, "Hello ** ********");
}

#[test]
fn numbers_become_hash_per_char() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "user 42 home 99");
    assert_eq!(out, "user ## home ##");
}

#[test]
fn numbers_redacted_even_in_force_mode() {
    let mut cfg = Config::default();
    cfg.max_words_len = 2;
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "click submit 42 today");
    assert_eq!(out, "***** ****** ## *****");
}

#[test]
fn punctuation_preserved() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "user, click submit!");
    assert_eq!(out, "user, click submit!");
}

#[test]
fn contractions_preserved() {
    let mut cfg = Config::default();
    cfg.max_words_len = 100;
    let allow = AllowLists::default();
    let out = scrub(
        &ctx(&cfg, &allow),
        "I'll click submit but don't save it. Let's continue.",
    );
    assert_eq!(out, "I'll click submit but don't save it. Let's continue.");
}

#[test]
fn typographic_apostrophe_handled() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "I\u{2019}ll save it");
    assert_eq!(out, "I\u{2019}ll save it");
}

#[test]
fn possessive_inherits_base_allow() {
    let cfg = Config::default();
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "the user's account");
    assert_eq!(out, "the user's account");
}

#[test]
fn force_redact_when_too_many_words() {
    let mut cfg = Config::default();
    cfg.max_words_len = 3;
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "click submit save cancel");
    assert_eq!(out, "***** ****** **** ******");
}

#[test]
fn under_word_count_threshold_allows_allowlisted() {
    let mut cfg = Config::default();
    cfg.max_words_len = 5;
    let allow = AllowLists::default();
    let out = scrub(&ctx(&cfg, &allow), "click submit cancel");
    assert_eq!(out, "click submit cancel");
}
