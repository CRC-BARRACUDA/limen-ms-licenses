//! Handing the last scan to a report provider.
//!
//! `report.build` is optional: when no provider is loaded the action is not
//! offered at all, so nothing here is ever reached by a user who has no way to
//! read the result.

use crate::*;

/// This machine's name, for the report's subtitle and its filename.
///
/// Read from the environment — no new permission, and no process spawned for a
/// string the system already has. Empty rather than a guess if it is not there:
/// a report labelled "unknown" is one somebody has to open to identify.
pub(crate) fn hostname() -> String {
    for var in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(name) = std::env::var(var) {
            if !name.trim().is_empty() {
                return name.trim().to_string();
            }
        }
    }
    String::new()
}

/// The last scan as a report provider expects it: a summary, a chart of
/// statuses, and the same tables the screen shows.
pub(crate) fn report_spec(lang: &str, lics: &[License], facts: &[Fact]) -> Value {
    report_spec_for(lang, lics, facts, &hostname())
}

/// [`report_spec`] for the machine named `host` ("" when it has no name to give).
pub(crate) fn report_spec_for(lang: &str, lics: &[License], facts: &[Fact], host: &str) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    let bad = lics.iter().filter(|l| !l.ok()).count();

    // Statuses and how many carry each, commonest first, then alphabetical so
    // the order is stable between scans rather than following a hash.
    let mut counts: Vec<(String, usize)> = Vec::new();
    for l in lics {
        match counts.iter_mut().find(|(s, _)| *s == l.status) {
            Some((_, n)) => *n += 1,
            None => counts.push((l.status.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let chart: Vec<Value> = counts
        .iter()
        .map(|(status, n)| {
            let label = if STATUS_NAMES.contains(&status.as_str()) {
                t(&format!("status.{status}"))
            } else {
                status.clone()
            };
            json!({ "label": label, "value": n })
        })
        .collect();

    let mut sections = vec![json!({
        "heading": t("ui.title"),
        "columns": columns(lang),
        "rows": lics.iter().map(|l| cells(lang, l)).collect::<Vec<_>>(),
    })];
    if !facts.is_empty() {
        sections.push(json!({
            "heading": t("ui.details"),
            "columns": [t("col.item"), t("col.value")],
            "rows": facts.iter()
                .map(|f| vec![f.item.clone(), f.value.clone()])
                .collect::<Vec<_>>(),
        }));
    }

    json!({
        "title": t("report.title"),
        // The machine this is about, in the line under the title and in the
        // name the file is offered under. The report module adds the date; a
        // folder of files all called "licenses" is one nobody can find
        // anything in.
        "subtitle": t(if host.is_empty() { "report.subtitle" } else { "report.subtitle_host" })
            .replace("{host}", host)
            .replace("{total}", &lics.len().to_string())
            .replace("{bad}", &bad.to_string()),
        "file_name": if host.is_empty() { "licenses".to_string() } else { format!("{host}_licenses") },
        // Always the provider's preview. What the document is — a PDF — the
        // ground it is printed on, and whether it is signed are decided there,
        // beside the thing being saved, by the module that owns the question.
        "format": "view",
        "summary": [
            t("report.line_total").replace("{n}", &lics.len().to_string()),
            t("report.line_bad").replace("{n}", &bad.to_string()),
        ],
        "charts": if chart.is_empty() {
            json!([])
        } else {
            json!([{ "title": t("report.by_status"), "data": chart }])
        },
        "sections": sections,
    })
}

/// What the screen says when the report has been handed over.
///
/// A provider that *renders* answers with a view, which is shown as it is —
/// the one shipped with Limen always does: its preview. One that only writes a
/// file acknowledges with nothing — so there has to be something to show for
/// it, or the user cannot tell success from silence.
pub(crate) fn delivered(lang: &str, built: Value) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    if built.get("widgets").is_some() {
        return built;
    }
    window(
        t("report.title"),
        vec![label(t("report.written")).strong()],
    )
}

/// What the screen says when the provider could not build it.
pub(crate) fn failed_view(lang: &str, why: &str) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    window(
        t("report.title"),
        vec![label(t("report.failed")).strong(), label(why).weak()],
    )
}

/// What it says when there is nothing to report on yet.
pub(crate) fn nothing_view(lang: &str) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    window(t("report.title"), vec![label(t("report.nothing")).strong()])
}
