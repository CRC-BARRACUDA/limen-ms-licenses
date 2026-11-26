//! Everything the module draws.
//!
//! Each screen takes `lang` and resolves its words through the catalog, so the
//! same call renders in whatever language the host is set to. Nothing here
//! holds state or asks the host anything: the handler decides *what* to show
//! and hands it in, which is what makes these testable without a host.

use crate::*;

/// The landing screen: nothing is read until the user asks for it.
pub(crate) fn idle_view(lang: &str) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    window(
        t("ui.title"),
        vec![
            label(t("ui.subtitle")).weak(),
            separator(),
            button(t("ui.scan"), CAP, "scan").primary(),
        ],
    )
}

/// What the screen shows when the store could not be read at all.
///
/// A way to try again is the point of it: the usual causes — the Software
/// Protection service stopped, a policy that blocked the run — are things the
/// user can put right and then ask again without hunting for the button.
pub(crate) fn error_view(lang: &str, message: &str) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    window(
        t("ui.title"),
        vec![
            label(message).weak(),
            separator(),
            button(t("ui.rescan"), CAP, "scan").primary(),
        ],
    )
}

/// Shown while the operating system is asking for privileges.
///
/// A pop-up rather than a line on the tab: an authorization prompt is a
/// question waiting to be answered, and it says which one field is being asked
/// for — a prompt whose reason is off-screen is a prompt people learn to click
/// through. It polls itself, so it leaves the moment the prompt is answered
/// either way.
pub(crate) fn authorizing_view(lang: &str) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    let mut v = window_modal_sized(
        t("oem.title"),
        "ms-licenses.authorizing",
        520.0,
        vec![
            step(t("oem.authorizing"), "loading"),
            label(t("oem.why")).weak(),
        ],
    );
    if let Value::Object(m) = &mut v {
        m.insert(
            "auto".into(),
            json!({ "capability": CAP, "method": "oem_poll", "args": {} }),
        );
    }
    v
}

/// The table's column headings, in order.
pub(crate) fn columns(lang: &str) -> Vec<String> {
    let t = |k: &str| catalog().tr(lang, k);
    vec![
        t("col.product"),
        t("col.status"),
        t("col.channel"),
        t("col.key"),
        t("col.expires"),
        t("col.kms"),
    ]
}

/// One licence as a row.
///
/// The key is shown as the store gives it — the last five characters — with the
/// rest masked, so the column reads as a fragment rather than as something
/// withheld. The full key is deliberately never recovered from the registry:
/// identifying a licence and handing over a credential are different jobs, and
/// only the first belongs in an inventory tool.
pub(crate) fn cells(lang: &str, l: &License) -> Vec<String> {
    let status = if STATUS_NAMES.contains(&l.status.as_str()) {
        catalog().tr(lang, &format!("status.{}", l.status))
    } else {
        // An unrecognised code, shown as itself rather than named something it
        // might not be.
        l.status.clone()
    };
    let key = if l.key.is_empty() {
        String::new()
    } else {
        format!("·····-{}", l.key)
    };
    vec![
        l.name.clone(),
        status,
        l.channel.clone(),
        key,
        l.expires.clone(),
        l.kms.clone(),
    ]
}

/// The results: a headline, whatever could not be read, and a table per product
/// family.
pub(crate) fn results_view(
    lang: &str,
    lics: &[License],
    facts: &[Fact],
    flags: &Flags,
    report: bool,
) -> Value {
    let t = |k: &str| catalog().tr(lang, k);
    let bad = lics.iter().filter(|l| !l.ok()).count();

    // The headline is the question people open this to answer, so it is the
    // first line rather than something to work out from the table.
    let mut widgets = vec![if bad == 0 {
        label(t("ui.all_ok")).weak()
    } else {
        label(t("ui.attention").replace("{n}", &bad.to_string())).weak()
    }];

    // What the run could not read. Said on the screen rather than logged,
    // because a missing field is part of the answer — and, for the one field
    // that only needs privileges, offered rather than merely reported.
    if flags.no_oem {
        widgets.push(label(format!("! {}", t("note.no_oem"))).weak());
        widgets.push(button(t("note.read_oem"), CAP, "read_oem"));
    }
    if !flags.error.is_empty() {
        widgets.push(label(format!("! {}", flags.error)).weak());
    }

    let mut actions = vec![button(t("ui.rescan"), CAP, "scan")];
    if report {
        actions.push(button(t("ui.report"), CAP, "make_report").open_in_tab());
    }
    widgets.push(row(actions));

    for (kind, heading) in [
        ("windows", "kind.windows"),
        ("office", "kind.office"),
        ("other", "kind.other"),
    ] {
        let rows: Vec<Vec<String>> = lics
            .iter()
            .filter(|l| l.kind == kind)
            .map(|l| cells(lang, l))
            .collect();
        if rows.is_empty() {
            continue;
        }
        widgets.push(separator());
        widgets.push(label(t(heading)).strong());
        widgets.push(table(columns(lang), rows));
    }

    if !facts.is_empty() {
        widgets.push(separator());
        widgets.push(label(t("ui.details")).strong());
        widgets.push(table(
            vec![t("col.item"), t("col.value")],
            facts
                .iter()
                .map(|f| vec![f.item.clone(), f.value.clone()])
                .collect(),
        ));
    }

    window(t("ui.title"), widgets)
}
