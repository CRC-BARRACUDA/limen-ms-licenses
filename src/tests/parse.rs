//! Reading back what the query printed.

use super::*;

/// The ordinary case: licences and facts come out, and the run is marked as
/// having started.
#[test]
fn a_run_yields_its_licences_and_its_facts() {
    let (lics, facts, flags) = parse(&sample());
    assert!(flags.ran, "the begin marker says the query got going");
    assert_eq!(lics.len(), 2);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].section, "windows");
    assert_eq!(facts[0].item, "Client machine id");
    assert_eq!(facts[0].value, "abc-123");
    assert!(flags.error.is_empty());
}

/// Without the begin marker the query never got going, and that is not the same
/// as a machine with nothing licensed. The handler turns this into an error
/// screen; here it is enough that the flag is not set.
#[test]
fn a_run_that_never_started_says_so() {
    let (lics, _, flags) = parse("");
    assert!(!flags.ran);
    assert!(lics.is_empty());
}

/// The OEM key needs administrator on some builds. Its absence is reported, not
/// left as a blank that reads like "there is no key".
#[test]
fn the_unreadable_oem_key_is_reported() {
    let (_, _, flags) = parse("#begin\n#no-oem\n");
    assert!(flags.no_oem);

    let (_, _, flags) = parse(&sample());
    assert!(!flags.no_oem, "nothing said means nothing missing");
}

/// Whatever the query itself raised is carried out rather than swallowed.
#[test]
fn an_error_from_the_query_is_carried_out() {
    let (_, _, flags) = parse("#begin\n#err=Access denied reading the service\n");
    assert_eq!(flags.error, "Access denied reading the service");
}

/// The status codes are the whole answer to "is this activated", so they are
/// named. Only `1` is actually licensed — a grace period is a deadline, not
/// activation.
#[test]
fn the_status_codes_are_named_and_only_one_is_licensed() {
    for (code, name) in [
        ("0", "unlicensed"),
        ("1", "licensed"),
        ("2", "grace"),
        ("3", "grace"),
        ("4", "non_genuine"),
        ("5", "notification"),
        ("6", "grace"),
    ] {
        assert_eq!(status_name(code), name, "status {code}");
    }
    for code in ["0", "2", "3", "4", "5", "6"] {
        let (lics, _, _) = parse(&format!("#begin\n{}", lic("windows", "W", code)));
        assert!(!lics[0].ok(), "status {code} is not licensed");
    }
    let (lics, _, _) = parse(&format!("#begin\n{}", lic("windows", "W", "1")));
    assert!(lics[0].ok());
}

/// A code nobody has seen before keeps its own number. Naming it something it
/// might not be would be worse than showing it: a number can be looked up.
#[test]
fn an_unknown_status_keeps_its_number() {
    let (lics, _, _) = parse(&format!("#begin\n{}", lic("windows", "W", "9")));
    assert_eq!(lics[0].status, "9");
    assert!(!lics[0].ok());
}

/// Windows first, then Office, then anything else — and within each family the
/// ones that are **not** licensed first, because they are why somebody opened
/// this.
#[test]
fn what_needs_attention_comes_first_within_its_family() {
    let out = [
        "#begin",
        &lic("other", "Some other product", "1"),
        &lic("office", "Office licensed", "1"),
        &lic("office", "Office unlicensed", "0"),
        &lic("windows", "Windows licensed", "1"),
        &lic("windows", "Windows grace", "2"),
    ]
    .join("\n");
    let (lics, _, _) = parse(&out);
    let names: Vec<&str> = lics.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Windows grace",
            "Windows licensed",
            "Office unlicensed",
            "Office licensed",
            "Some other product",
        ]
    );
}

/// The fields that carry the interesting detail survive the trip.
#[test]
fn the_kms_host_and_the_grace_period_survive() {
    let line = "lic|windows|Windows|2|Volume:MAK|T3FG2|17 days|kms.example.org:1688|desc";
    let (lics, _, _) = parse(&format!("#begin\n{line}"));
    let l = &lics[0];
    assert_eq!(l.channel, "Volume:MAK");
    assert_eq!(l.expires, "17 days");
    assert_eq!(l.kms, "kms.example.org:1688");
    assert_eq!(l.description, "desc");
    assert_eq!(l.status, "grace");
}

/// Only the last five characters of a key are ever in the record — that is all
/// the store holds, and all this module ever wants.
#[test]
fn only_the_partial_key_is_ever_present() {
    let (lics, _, _) = parse(&sample());
    for l in &lics {
        assert_eq!(l.key.len(), 5, "{:?} is not a partial key", l.key);
    }
    // And the query never asks for more than that: the full key sits in the
    // registry under a name this module must not go near.
    assert!(
        !query().contains("DigitalProductId") && !query().contains("BackupProductKeyDefault"),
        "the query must not reach for the full key"
    );
}

/// A record is one line, and a line that is not a record is skipped rather than
/// guessed at. PowerShell writes the odd blank line and the odd warning, and
/// neither is a licence.
#[test]
fn lines_that_are_not_records_are_left_alone() {
    let out = [
        "#begin",
        "",
        "   ",
        "WARNING: something the shell wanted to say",
        "lic|windows|Too|few|fields",
        &lic("windows", "Real", "1"),
    ]
    .join("\n");
    let (lics, facts, flags) = parse(&out);
    assert!(flags.ran);
    assert!(facts.is_empty());
    assert_eq!(lics.len(), 1, "only the whole record is a licence");
    assert_eq!(lics[0].name, "Real");
}

/// The query strips `|` out of every value before printing, so a record can
/// never be split by its own content — but if one ever arrived, the value
/// should come back whole rather than truncated at the separator.
#[test]
fn a_value_holding_a_separator_arrives_whole() {
    let (_, facts, _) = parse("#begin\nfact|windows|Item|a|b|c");
    assert_eq!(facts[0].value, "a|b|c");
}
