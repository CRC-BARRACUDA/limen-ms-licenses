//! What this module is expected to do, in the language of what it is for.
//!
//! The PowerShell query itself cannot run here — it is Windows only, which is
//! the whole point of the module — so what is tested is everything around it:
//! the parsing of what it prints, the order things come out in, the screens,
//! and that every string the code asks for exists in both languages.

use crate::*;

mod i18n;
mod oem;
mod parse;
mod report;
mod views;

/// One licence line, as the query writes it: nine fields, of which the last
/// three — expiry, KMS host, description — are empty on an ordinary product.
fn lic(kind: &str, name: &str, status: &str) -> String {
    format!("lic|{kind}|{name}|{status}|Retail|T3FG2|||")
}

/// A whole run: the begin marker, some licences, and a fact.
fn sample() -> String {
    [
        "#begin",
        &lic("windows", "Windows(R), Professional edition", "1"),
        &lic("office", "Office 16, Office16ProPlusVL_KMS_Client edition", "2"),
        "fact|windows|Client machine id|abc-123",
        "",
    ]
    .join("\n")
}
