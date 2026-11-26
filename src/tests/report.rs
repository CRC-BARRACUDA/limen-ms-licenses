//! What a report provider is handed, and what comes back.

use super::*;

/// The report carries the same rows the screen shows, plus the counts a reader
/// wants before the tables.
#[test]
fn the_report_carries_the_scan_and_says_what_is_in_it() {
    let (lics, facts, _) = parse(&format!(
        "#begin\n{}\n{}\nfact|windows|Client machine id|abc-123",
        lic("windows", "Windows Pro", "1"),
        lic("office", "Office 16", "0")
    ));
    let spec = report_spec("en", &lics, &facts);

    assert_eq!(spec["format"], json!("view"));
    assert_eq!(spec["title"], json!("Licence report"));
    let s = spec.to_string();
    assert!(s.contains("Products found: 2"), "{s}");
    assert!(s.contains("Not properly licensed: 1"), "{s}");
    // Both tables: the licences, and the facts.
    assert_eq!(spec["sections"].as_array().unwrap().len(), 2);
    assert!(s.contains("Windows Pro") && s.contains("Office 16"));
    assert!(s.contains("abc-123"));
}

/// The chart counts statuses, commonest first, and names them the way the
/// screen does.
#[test]
fn the_chart_counts_each_status_by_name() {
    let (lics, facts, _) = parse(&format!(
        "#begin\n{}\n{}\n{}",
        lic("windows", "A", "1"),
        lic("office", "B", "1"),
        lic("office", "C", "0")
    ));
    let spec = report_spec("en", &lics, &facts);
    let data = &spec["charts"][0]["data"];
    assert_eq!(data[0]["label"], json!("Licensed"));
    assert_eq!(data[0]["value"], json!(2));
    assert_eq!(data[1]["label"], json!("Not licensed"));
    assert_eq!(data[1]["value"], json!(1));
}

/// Nothing scanned, nothing to chart — and no empty chart frame on the page.
#[test]
fn a_report_of_nothing_carries_no_chart() {
    let spec = report_spec("en", &[], &[]);
    assert_eq!(spec["charts"], json!([]));
    assert_eq!(spec["sections"].as_array().unwrap().len(), 1);
}

/// A provider that renders answers with a view, and that view is what the user
/// sees — this module must not wrap it in a second window.
#[test]
fn a_rendered_report_is_shown_as_it_came_back() {
    let built = json!({ "title": "Theirs", "widgets": [] });
    assert_eq!(delivered("en", built.clone()), built);
}

/// A provider that exports writes a file and acknowledges with nothing. Left
/// alone, that is a tab that opens empty — so there has to be something saying
/// what happened.
#[test]
fn an_exported_report_still_says_something() {
    let v = delivered("en", json!({ "ok": true })).to_string();
    assert!(v.contains("Report exported"), "{v}");
    assert!(v.contains("opened in your default app"), "{v}");
}

/// Both failure screens say which failure it was.
#[test]
fn a_failure_says_which_failure_it_was() {
    let v = failed_view("en", "no provider answered").to_string();
    assert!(v.contains("Couldn't build the report"), "{v}");
    assert!(v.contains("no provider answered"), "{v}");

    let v = nothing_view("en").to_string();
    assert!(v.contains("Nothing has been checked yet"), "{v}");
}
