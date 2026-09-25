//! `ms-licenses` — how Windows and Office are activated on **this** machine.
//!
//! Status, channel, the partial key, the KMS host it talks to, and what is
//! running on a grace period. The `slmgr /dlv` answer, without the four pop-ups.
//!
//! Provides `licenses.ms`. Methods: `licenses` (the last scan as data, for
//! other modules), plus `ui` / `scan` / `make_report` for the built-in view.
//! The UI does **not** read anything on open — it reads when the user presses
//! the button, and shows the last result thereafter.
//!
//! Windows only, declared in the manifest as `os = ["windows"]`, which the host
//! enforces: a module that names platforms and not the running one is dropped
//! at discovery, so this never appears in the module manager elsewhere.
//!
//! What it reads
//! -------------
//! `SoftwareLicensingProduct` carries both Windows and Office on a modern
//! system, told apart by their application id. Office 2010-2013 predates that
//! and lives in `OfficeSoftwareProtectionProduct`, which is read as well when it
//! exists. Click-to-Run Office keeps its channel and build in the registry
//! rather than in either class, so that is read too — a subscription install
//! often has no product key at all, and the channel is the only thing that
//! identifies it.
//!
//! It reads. Nothing is activated, rearmed, installed or removed, and nothing
//! leaves the machine.
//!
//! A note on keys
//! --------------
//! Only the **last five characters** of a product key are shown — that is all
//! the licensing store holds, and it is enough to tell two licences apart
//! without being a key anybody could use. The full key is deliberately not
//! recovered from the registry: identifying a licence and handing over a
//! credential are different jobs, and only the first belongs in an inventory
//! tool.
//!
//! ```text
//!   store    running the query, and reading back what it printed
//!   license  one licence, what its status code means, and what order they go in
//!   view     everything the module draws
//!   report   handing the last scan to a report provider
//!   handler  what a call does, and what is remembered between calls
//! ```
//!
//! Built as a native (`cdylib`) module using `limen-sdk-rust`.

mod handler;
mod license;
mod report;
mod store;
mod view;

#[cfg(test)]
mod tests;

// This reads best as one namespace: each part takes `use crate::*` and finds
// everything, rather than every file carrying a list of its neighbours that has
// to be maintained by hand.
pub(crate) use handler::*;
pub(crate) use license::*;
pub(crate) use report::*;
pub(crate) use store::*;
pub(crate) use view::*;

pub(crate) use limen_sdk_rust::ui::{
    button, label, notice, row, separator, step, table, window, window_modal_sized,
};
pub(crate) use limen_sdk_rust::{json, rpc, Catalog, Handler, Host, RpcError, Value};

use limen_sdk_rust::export_module;

/// The capability this module provides. Named once: it is also the target every
/// button on every screen calls back into.
pub(crate) const CAP: &str = "licenses.ms";

/// This module's own translations (its `locales/*.toml`, embedded at compile
/// time). English is the default and the fallback; `host.locale()` selects the
/// active one at render time.
pub(crate) fn catalog() -> &'static Catalog {
    static C: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        Catalog::new(&[
            ("en", include_str!("../locales/en.toml")),
            ("uk", include_str!("../locales/uk.toml")),
        ])
    })
}

export_module!(MsLicenses);
