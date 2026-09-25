//! The screens: what they show, and what they refuse to claim.

use super::*;

/// Nothing is read until the user asks. The landing screen is a button and a
/// sentence saying what pressing it will do.
#[test]
fn nothing_is_read_until_it_is_asked_for() {
    let v = idle_view("en");
    let s = v.to_string();
    assert_eq!(v["title"], json!("Windows & Office licences"));
    assert!(s.contains("Check licences"));
    assert!(s.contains("Nothing is activated"), "{s}");
    // No table: there is nothing to put in one yet.
    assert!(!s.contains("\"kind\":\"table\""));
}

/// The headline answers the question people opened this to ask, rather than
/// leaving them to work it out from the table.
#[test]
fn the_headline_says_whether_anything_needs_attention() {
    let (ok, facts, flags) = parse(&format!("#begin\n{}", lic("windows", "W", "1")));
    let v = results_view("en", &ok, &facts, &flags, false).to_string();
    assert!(v.contains("Everything found is licensed"), "{v}");

    let (bad, facts, flags) = parse(&format!(
        "#begin\n{}\n{}",
        lic("windows", "W", "0"),
        lic("office", "O", "2")
    ));
    let v = results_view("en", &bad, &facts, &flags, false).to_string();
    assert!(v.contains("2 product(s) need attention"), "{v}");
}

/// One table per product family, and no empty heading for a family that is not
/// installed.
#[test]
fn each_product_family_gets_its_own_table() {
    let (lics, facts, flags) = parse(&format!(
        "#begin\n{}\n{}",
        lic("windows", "Windows Pro", "1"),
        lic("office", "Office 16", "1")
    ));
    let v = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(v.contains("Windows"), "{v}");
    assert!(v.contains("Office"), "{v}");
    // Nothing under "other", so that heading must not appear.
    assert!(!v.contains("Other products"), "{v}");
}

/// The key column is a fragment, shown as one. The last five characters are all
/// the store holds; the mask says so rather than looking like something
/// withheld.
#[test]
fn the_key_is_shown_as_the_fragment_it_is() {
    let (lics, _, _) = parse(&sample());
    assert_eq!(cells("en", &lics[0])[3], "·····-T3FG2");

    // A product with no key at all gets an empty cell, not a mask with nothing
    // behind it.
    let line = "lic|windows|W|1|Retail||||";
    let (none, _, _) = parse(&format!("#begin\n{line}"));
    assert_eq!(cells("en", &none[0])[3], "");
}

/// A status is named on screen; an unrecognised code is shown as itself.
#[test]
fn the_status_column_is_words_unless_it_cannot_be() {
    let (lics, _, _) = parse(&format!("#begin\n{}", lic("windows", "W", "2")));
    assert_eq!(cells("en", &lics[0])[1], "Grace period");

    let (odd, _, _) = parse(&format!("#begin\n{}", lic("windows", "W", "9")));
    assert_eq!(cells("en", &odd[0])[1], "9");
}

/// What could not be read is said out loud. A missing field is part of the
/// answer, not something to leave blank.
#[test]
fn what_could_not_be_read_is_said_on_the_screen() {
    let (lics, facts, flags) = parse(&format!("#begin\n#no-oem\n{}", lic("windows", "W", "1")));
    let v = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(v.contains("OEM key held in firmware could not be read"), "{v}");

    let (lics, facts, flags) = parse(&format!(
        "#begin\n#err=the service is stopped\n{}",
        lic("windows", "W", "1")
    ));
    let v = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(v.contains("the service is stopped"), "{v}");
}

/// The report action belongs to a provider that is loaded. Offering it with no
/// provider would be a button that can only disappoint.
#[test]
fn the_report_action_appears_only_with_a_provider() {
    let (lics, facts, flags) = parse(&sample());
    let without = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(!without.contains("Make report"), "{without}");

    let with = results_view("en", &lics, &facts, &flags, true).to_string();
    assert!(with.contains("Make report"), "{with}");
}

/// A screen that says the store could not be read has to offer a way to try
/// again — the usual causes are things the user can put right.
#[test]
fn an_error_screen_is_a_screen_you_can_leave() {
    let v = error_view("en", "PowerShell could not be run").to_string();
    assert!(v.contains("PowerShell could not be run"), "{v}");
    assert!(v.contains("Check again"), "{v}");
}

/// The facts table is shown when there is something in it, and not when there
/// is not.
#[test]
fn the_details_table_appears_only_when_there_are_details() {
    let (lics, facts, flags) = parse(&sample());
    let v = results_view("en", &lics, &facts, &flags, false).to_string();
    assert!(v.contains("Details"), "{v}");
    assert!(v.contains("abc-123"), "{v}");

    let (lics, _, flags) = parse(&format!("#begin\n{}", lic("windows", "W", "1")));
    let v = results_view("en", &lics, &[], &flags, false).to_string();
    assert!(!v.contains("Details"), "{v}");
}

/// The screen follows the host's language — the title, and the table's own
/// content, not merely its headings.
#[test]
fn the_screen_follows_the_host_language() {
    let (lics, facts, flags) = parse(&format!("#begin\n{}", lic("windows", "W", "2")));
    let v = results_view("uk", &lics, &facts, &flags, true).to_string();
    assert!(v.contains("Ліцензії"), "the title is translated: {v}");
    assert!(v.contains("Пільговий"), "so is a status: {v}");
    assert!(!v.contains("Grace period"), "nothing English is left: {v}");
}
