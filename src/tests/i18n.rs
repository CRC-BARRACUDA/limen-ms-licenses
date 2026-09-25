//! That every string the code asks for exists, in both languages.

use super::*;

/// The keys a catalog file defines, as `section.item`.
fn keys(toml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut section = String::new();
    for line in toml.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_string();
        } else if let Some((k, _)) = line.split_once('=') {
            let k = k.trim();
            if !k.is_empty() && !k.starts_with('#') {
                out.push(format!("{section}.{k}"));
            }
        }
    }
    out
}

/// Ukrainian must answer every key English does. A key missing from `uk.toml`
/// falls back to English, which is a screen that is half translated — worse
/// than one that is not, because nobody notices.
#[test]
fn ukrainian_covers_every_english_key() {
    let en = keys(include_str!("../../locales/en.toml"));
    let uk = keys(include_str!("../../locales/uk.toml"));
    let missing: Vec<&String> = en.iter().filter(|k| !uk.contains(k)).collect();
    assert!(missing.is_empty(), "uk.toml is missing: {missing:?}");

    // And nothing in uk.toml that English does not have — a key nobody reads
    // is a translation somebody paid for and a line nobody will ever delete.
    let extra: Vec<&String> = uk.iter().filter(|k| !en.contains(k)).collect();
    assert!(extra.is_empty(), "uk.toml carries keys nothing asks for: {extra:?}");
}

/// No screen shows a bare key. A key that reaches the screen untranslated is
/// the failure this catches — it renders as `ui.title`, which looks like a bug
/// to a user and like nothing at all to a test that only checks it is present.
#[test]
fn no_screen_shows_a_bare_translation_key() {
    let (lics, facts, flags) = parse(&format!(
        "#begin\n#no-oem\n{}\n{}\nfact|windows|Client machine id|abc",
        lic("windows", "W", "1"),
        lic("other", "Something else", "0")
    ));
    for lang in ["en", "uk"] {
        let screens = [
            idle_view(lang),
            error_view(lang, "why"),
            results_view(lang, &lics, &facts, &flags, true),
            nothing_view(lang),
            failed_view(lang, "why"),
            report_spec(lang, &lics, &facts),
        ];
        for screen in screens {
            let s = screen.to_string();
            for key in keys(include_str!("../../locales/en.toml")) {
                // `[module]` is the host's to read, for the card — the module
                // never draws it.
                if key.starts_with("module.") {
                    continue;
                }
                assert!(
                    !s.contains(&format!("\"{key}\"")),
                    "{lang}: untranslated key on screen: {key}"
                );
            }
        }
    }
}

/// The counts that go in a sentence actually get there. `{n}` left in the text
/// is a screen that says "{n} product(s) need attention".
#[test]
fn the_numbers_reach_the_sentences_that_hold_them() {
    let (lics, facts, flags) = parse(&format!(
        "#begin\n{}\n{}",
        lic("windows", "W", "0"),
        lic("office", "O", "1")
    ));
    for lang in ["en", "uk"] {
        let v = results_view(lang, &lics, &facts, &flags, false).to_string();
        assert!(v.contains('1'), "{lang}: the count is missing: {v}");
        assert!(!v.contains("{n}"), "{lang}: a placeholder survived: {v}");

        let r = report_spec(lang, &lics, &facts).to_string();
        assert!(!r.contains("{n}"), "{lang}: {r}");
        assert!(!r.contains("{total}") && !r.contains("{bad}"), "{lang}: {r}");
    }
}

/// The two languages must actually differ. A uk.toml that is a copy of en.toml
/// passes every key check and translates nothing.
#[test]
fn the_translation_is_a_translation() {
    for key in ["ui.title", "ui.scan", "status.licensed", "report.title"] {
        assert_ne!(
            catalog().tr("en", key),
            catalog().tr("uk", key),
            "{key} is the same in both languages"
        );
    }
}
