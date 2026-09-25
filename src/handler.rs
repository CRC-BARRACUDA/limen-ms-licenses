//! The module itself — what it holds between calls, and what each call does.

use crate::*;

/// The last scan. Held so reopening the tab shows what was found rather than
/// setting a PowerShell process going on every draw.
#[derive(Default)]
pub struct MsLicenses {
    /// `None` until a scan has succeeded. Distinct from an empty list, which
    /// would be a machine on which nothing is licensed — a real answer, and a
    /// very different one from "not read yet".
    lics: Option<Vec<License>>,
    facts: Vec<Fact>,
    flags: Flags,
    /// Why the last scan produced nothing, ready to show and to hand to a
    /// caller of `licenses` that arrives before any successful scan.
    error: String,
    /// The elevated OEM-key read that is in flight, if one is: the host's id
    /// for it, and the file it was told to write.
    oem: Option<(u64, std::path::PathBuf)>,
}

impl MsLicenses {
    /// Read the licensing store and render the result.
    fn scan(&mut self, lang: &str, host: &Host, report: bool) -> Value {
        match read_store() {
            Ok(stdout) => {
                let (lics, facts, flags) = parse(&stdout);
                if !flags.ran {
                    // No marker means the query never got going — PowerShell
                    // missing, or blocked by policy. Reporting an empty list
                    // here would read as "nothing is licensed".
                    self.fail(catalog().tr(lang, "error.no_powershell"));
                } else {
                    host.log(&format!(
                        "ms-licenses: {} licence(s), {} fact(s)",
                        lics.len(),
                        facts.len()
                    ));
                    self.lics = Some(lics);
                    self.facts = facts;
                    self.flags = flags;
                    self.error.clear();
                }
            }
            Err(ReadError::NoPowerShell) => {
                self.fail(catalog().tr(lang, "error.no_powershell"))
            }
            Err(ReadError::Other(e)) => {
                host.log(&format!("ms-licenses: scan failed: {e}"));
                self.fail(e);
            }
        }
        self.render(lang, report)
    }

    /// Forget the last scan and remember why.
    ///
    /// The previous results are dropped rather than left on screen under an
    /// error: a licence table from ten minutes ago, shown beside a message
    /// saying the store could not be read, invites reading stale state as
    /// current.
    fn fail(&mut self, why: String) {
        self.lics = None;
        self.facts.clear();
        self.flags = Flags::default();
        self.error = why;
    }

    /// Whatever the module should be showing right now.
    fn render(&self, lang: &str, report: bool) -> Value {
        match &self.lics {
            Some(lics) => results_view(lang, lics, &self.facts, &self.flags, report),
            None if !self.error.is_empty() => error_view(lang, &self.error),
            None => idle_view(lang),
        }
    }

    /// Ask the operating system to read the one field that needs privileges.
    ///
    /// Started rather than waited on: the prompt can sit there indefinitely —
    /// the user may be looking at a password dialog, or may have walked away —
    /// and waiting would block every other call this module makes, including
    /// the one that draws the screen saying what is being waited for.
    fn read_oem(&mut self, lang: &str, host: &Host, report: bool) -> Value {
        let Some(dir) = host.module_dir() else {
            // Nowhere of our own to have the key written. Not worth a special
            // screen: it means the host could not say where this module lives.
            return notice(
                self.render(lang, report),
                "error",
                catalog().tr(lang, "oem.failed"),
            );
        };
        let file = oem_file(&dir);
        // Anything a previous attempt left behind, so what is read back is
        // certainly from this run and not from the last one.
        let _ = std::fs::remove_file(&file);
        let argv = oem_argv(&file);
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        match host.elevate_async(&argv, Some(&dir)) {
            Some(id) => {
                self.oem = Some((id, file));
                authorizing_view(lang)
            }
            // `elevate_async` answers `None` for every refusal alike — a
            // platform that cannot ask, a manifest that did not declare it.
            // The honest message is the one that does not guess which.
            None => notice(
                self.render(lang, report),
                "warning",
                catalog().tr(lang, "oem.unavailable"),
            ),
        }
    }

    /// Where that read has got to. Called by the pop-up, on repeat, until it
    /// has an answer.
    fn oem_poll(&mut self, lang: &str, host: &Host, report: bool) -> Value {
        let t = |k: &str| catalog().tr(lang, k);
        let Some((id, file)) = self.oem.clone() else {
            // Nothing in flight: the pop-up outlived what it was watching.
            return self.render(lang, report);
        };
        match host.elevate_state(id) {
            // Still at the prompt, or running: the same pop-up, which polls
            // again. The two are one screen here because the command itself
            // takes a moment — it is the prompt that takes the time.
            limen_sdk_rust::ElevateState::Authorizing
            | limen_sdk_rust::ElevateState::Running => authorizing_view(lang),
            limen_sdk_rust::ElevateState::Done(done) => {
                self.oem = None;
                let key = take_oem(&file);
                if !done.ok() {
                    // Said in the user's language from the fixed word, not from
                    // the English detail, which is for the log.
                    let why = if done.refused() {
                        t("oem.refused")
                    } else if done.unavailable() {
                        t("oem.unavailable")
                    } else {
                        t("oem.failed")
                    };
                    host.log(&format!("ms-licenses: OEM key not read: {}", done.message));
                    let level = if done.refused() { "info" } else { "warning" };
                    return notice(self.render(lang, report), level, why);
                }
                match key {
                    Some(key) => {
                        // It is a fact like the others now, and the note that
                        // said it could not be read no longer applies.
                        self.facts.push(Fact {
                            section: "windows".into(),
                            item: t("oem.item"),
                            value: key,
                        });
                        self.flags.no_oem = false;
                        self.render(lang, report)
                    }
                    // It ran, it was allowed, and there was nothing there —
                    // which is an answer, and a different one from a refusal.
                    None => {
                        self.flags.no_oem = false;
                        notice(self.render(lang, report), "ok", t("oem.none"))
                    }
                }
            }
        }
    }

    /// The capability method: the last scan as data, for other modules.
    ///
    /// Reports what was last read rather than reading again — a caller asking a
    /// question should not set a PowerShell process going on someone's machine.
    ///
    /// `{"kind": "office"}` narrows to one product family, and `{"ok": false}`
    /// to whatever is not properly licensed, which is the question a fleet
    /// report actually asks.
    fn licenses(&self, params: &Value) -> Value {
        let Some(lics) = &self.lics else {
            return json!({
                "error": if self.error.is_empty() { "nothing scanned yet" } else { &self.error },
                "total": 0,
                "licenses": [],
            });
        };
        let kind = params
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let ok = params.get("ok").and_then(Value::as_bool);
        let rows: Vec<&License> = lics
            .iter()
            .filter(|l| kind.is_empty() || l.kind == kind)
            .filter(|l| ok.is_none_or(|want| l.ok() == want))
            .collect();
        json!({
            "note": "Activation state of Windows and Office on this machine. \
                     `key` is the last five characters, which is all the licensing \
                     store holds. `status` is the licensing store's own state, \
                     named: licensed, unlicensed, grace, notification, non_genuine.",
            "total": rows.len(),
            "licensed": rows.iter().filter(|l| l.ok()).count(),
            "licenses": rows.iter().map(|l| l.to_json()).collect::<Vec<_>>(),
            "facts": self.facts.iter()
                .map(|f| json!({ "section": f.section, "item": f.item, "value": f.value }))
                .collect::<Vec<_>>(),
        })
    }

    /// Hand the last scan to whatever report provider is installed.
    fn make_report(&self, lang: &str, host: &Host) -> Value {
        let Some(lics) = &self.lics else {
            return nothing_view(lang);
        };
        let spec = report_spec(lang, lics, &self.facts);
        match host.call("report.build", "build", spec) {
            Ok(v) => delivered(lang, v),
            Err(e) => failed_view(lang, &e.message),
        }
    }
}

impl Handler for MsLicenses {
    fn capabilities(&self) -> Vec<String> {
        vec![CAP.into()]
    }

    fn invoke(
        &mut self,
        _capability: &str,
        method: &str,
        params: Value,
        host: &Host,
    ) -> Result<Value, RpcError> {
        // One lookup per call, then every view renders in that language.
        let lang = host.locale();
        let lang = lang.as_str();
        // Optional integration: the action appears only when a provider is
        // actually loaded. Asked each call rather than cached, so a provider
        // installed while this tab is open makes the button appear.
        let report = host.has_capability("report.build");
        match method {
            // Reopening the tab shows the last scan; nothing is read on open.
            "ui" => Ok(self.render(lang, report)),
            "scan" => Ok(self.scan(lang, host, report)),
            "read_oem" => Ok(self.read_oem(lang, host, report)),
            "oem_poll" => Ok(self.oem_poll(lang, host, report)),
            "licenses" => Ok(self.licenses(&params)),
            "make_report" => Ok(self.make_report(lang, host)),
            other => Err(RpcError::new(
                rpc::METHOD_NOT_FOUND,
                format!("No method {other}"),
            )),
        }
    }
}
