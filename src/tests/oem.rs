//! The one field that needs administrator, and what is asked for to get it.

use super::*;

/// A scan that could not read the OEM key offers to go and get it, rather than
/// only reporting that it could not.
#[test]
fn the_unreadable_field_comes_with_a_way_to_read_it() {
    let (lics, facts, flags) = parse(&format!("#begin\n#no-oem\n{}", lic("windows", "W", "1")));
    let v = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(v.contains("could not be read"), "{v}");
    assert!(v.contains("Read it as administrator"), "{v}");
    assert!(v.contains("read_oem"), "the button calls the method: {v}");

    // Nothing to offer when the field was read without privileges.
    let (lics, facts, flags) = parse(&format!("#begin\n{}", lic("windows", "W", "1")));
    let v = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(!v.contains("Read it as administrator"), "{v}");
}

/// The screen shown while the prompt is up says what is being asked for, and
/// polls itself so it leaves when the prompt is answered either way.
#[test]
fn the_prompt_screen_explains_itself_and_does_not_get_stuck() {
    let v = authorizing_view("en");
    let s = v.to_string();
    assert_eq!(v["modal"], json!("ms-licenses.authorizing"));
    assert!(s.contains("Waiting for you to allow it"), "{s}");
    // Which field, and that nothing else is elevated — a prompt whose reason is
    // off-screen is one people learn to click through.
    assert!(s.contains("OEM key"), "{s}");
    assert!(s.contains("nothing else is elevated"), "{s}");
    assert_eq!(v["auto"]["method"], json!("oem_poll"), "it polls itself");
    assert_eq!(v["auto"]["capability"], json!(CAP));
}

/// Exactly one property is asked for with privileges — not the whole query
/// again. What needs administrator is one field, so that is all that is run
/// with it.
#[test]
fn only_the_one_property_is_run_with_privileges() {
    let file = std::path::Path::new(r"C:\base\modules\ms-licenses\.oem-key");
    let argv = oem_argv(file);
    let script = argv.last().expect("the command");

    assert!(script.contains("OA3xOriginalProductKey"), "{script}");
    // Not the licensing product classes, and nothing that changes anything.
    assert!(!script.contains("SoftwareLicensingProduct"), "{script}");
    for forbidden in ["Set-CimInstance", "InstallProductKey", "Activate", "Rearm"] {
        assert!(!script.contains(forbidden), "{forbidden} in: {script}");
    }
}

/// The command is named by its full path. Elevation replaces the environment
/// with a minimal one, so a `PATH` lookup would not find it — and a `PATH`
/// lookup is not what should decide which binary runs as administrator.
#[test]
fn what_is_elevated_is_named_absolutely() {
    let argv = oem_argv(std::path::Path::new(r"C:\base\.oem-key"));
    let exe = &argv[0];
    assert!(exe.ends_with("powershell.exe"), "{exe}");
    assert!(
        exe.contains(r"\System32\WindowsPowerShell\"),
        "the full path, not a name to look up: {exe}"
    );
    assert!(argv.contains(&"-NoProfile".to_string()), "{argv:?}");
    assert!(argv.contains(&"-NonInteractive".to_string()), "{argv:?}");
}

/// The key comes back through a file — an elevated command's output does not
/// return — and the file does not outlive the reading of it.
#[test]
fn the_key_is_read_back_once_and_then_it_is_gone() {
    let dir = std::env::temp_dir().join("ms-licenses-oem-test");
    let _ = std::fs::create_dir_all(&dir);
    let file = oem_file(&dir.to_string_lossy());

    std::fs::write(&file, "ABCDE-FGHIJ-KLMNO-PQRST-UVWXY\r\n").unwrap();
    assert_eq!(
        take_oem(&file).as_deref(),
        Some("ABCDE-FGHIJ-KLMNO-PQRST-UVWXY"),
        "read back, without the line ending"
    );
    assert!(!file.exists(), "a product key must not be left on disk");

    // Read twice is not read once: there is nothing there the second time.
    assert_eq!(take_oem(&file), None);
}

/// A machine with no OEM key in firmware writes an empty file. That is an
/// answer — "there is none" — and not the same as "it could not be read", so
/// it must not come back looking like a key.
#[test]
fn an_empty_answer_is_not_a_key() {
    let dir = std::env::temp_dir().join("ms-licenses-oem-empty");
    let _ = std::fs::create_dir_all(&dir);
    let file = oem_file(&dir.to_string_lossy());

    for written in ["", "   ", "\r\n"] {
        std::fs::write(&file, written).unwrap();
        assert_eq!(take_oem(&file), None, "{written:?} is not a key");
        assert!(!file.exists());
    }
    // And a run that wrote nothing at all.
    assert_eq!(take_oem(&file), None);
}

/// The file is written where this module lives, not in the shared temp
/// directory: everything on the machine can write to temp, and the value is a
/// product key.
#[test]
fn the_key_is_written_somewhere_of_our_own() {
    let file = oem_file(r"C:\limen\modules\ms-licenses");
    assert!(
        file.starts_with(r"C:\limen\modules\ms-licenses"),
        "{}",
        file.display()
    );
    let argv = oem_argv(&file);
    let script = argv.last().unwrap();
    assert!(
        script.contains(&file.to_string_lossy().to_string()),
        "the command writes to that file: {script}"
    );
}

/// Every word this flow can say exists in both languages — the whole of it
/// happens while the user is waiting on a prompt, which is the worst moment to
/// meet a bare key or an English sentence.
#[test]
fn the_elevated_flow_speaks_both_languages() {
    for lang in ["en", "uk"] {
        let s = authorizing_view(lang).to_string();
        for key in ["oem.title", "oem.authorizing", "oem.why"] {
            assert!(!s.contains(key), "{lang}: bare key {key} on screen: {s}");
        }
    }
    for key in ["oem.refused", "oem.unavailable", "oem.failed", "oem.none", "oem.item", "note.read_oem"] {
        let en = catalog().tr("en", key);
        let uk = catalog().tr("uk", key);
        assert_ne!(en, key, "en is missing {key}");
        assert_ne!(uk, en, "{key} is not translated");
    }
}
