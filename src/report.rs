//! Handing the last scan to a report provider.
//!
//! `report.build` is optional: when no provider is loaded the action is not
//! offered at all, so nothing here is ever reached by a user who has no way to
//! read the result.

use crate::*;

/// The last scan as a report provider expects it: a summary, a chart of
/// statuses, and the same tables the screen shows.
pub(crate) fn report_spec(lang: &str, lics: &[License], facts: &[Fact]) -> Value {
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
        "subtitle": t("report.subtitle")
            .replace("{total}", &lics.len().to_string())
            .replace("{bad}", &bad.to_string()),
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
/// A provider that *renders* answers with a view, which is shown as it is. One
/// that *exports* writes a file, opens it, and acknowledges with nothing — so
/// there has to be something to show for it, or the tab opens empty and the
/// user cannot tell success from silence.
pub(crate) fn delivered(lang: &str, built: Value) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    if built.get("widgets").is_some() {
        return built;
    }
    window(
        t("report.title"),
        vec![
            label(t("report.exported")).strong(),
            label(t("report.exported_hint")).weak(),
        ],
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
