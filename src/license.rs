//! One licence as the store reports it, and what its numbers mean.

use crate::*;

/// A product the licensing store holds a licence for.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct License {
    /// `windows`, `office`, or `other` — which family the product belongs to.
    pub kind: String,
    pub name: String,
    /// The store's `LicenseStatus`, **named** (see [`status_name`]). An
    /// unrecognised code keeps its number.
    pub status: String,
    pub channel: String,
    /// The last five characters of the product key — all the store holds.
    pub key: String,
    /// How long a grace period has left, already in days.
    pub expires: String,
    /// The KMS host this product activates against, with its port if there is
    /// one, or a discovered host marked as such.
    pub kms: String,
    pub description: String,
}

impl License {
    /// Whether this product is properly licensed. Only status 1 is — a grace
    /// period is not activation, it is a deadline.
    pub fn ok(&self) -> bool {
        self.status == "licensed"
    }

    /// The record as other modules receive it.
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind,
            "name": self.name,
            "status": self.status,
            "channel": self.channel,
            "key": self.key,
            "expires": self.expires,
            "kms": self.kms,
            "description": self.description,
            "ok": self.ok(),
        })
    }
}

/// One `Item`/`Value` pair from the store that is not a licence — the client
/// machine id, the configured KMS host, the Click-to-Run channel.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Fact {
    /// `windows` or `office`: which table it is shown under.
    pub section: String,
    pub item: String,
    pub value: String,
}

/// `LicenseStatus` as the licensing store reports it.
///
/// The numbers are the whole answer to "is this machine activated", so they are
/// named rather than shown: `1` is not an answer anybody can read. The three
/// grace codes differ in *why* there is a grace period — out of the box, out of
/// tolerance after a hardware change, extended — which is a distinction the
/// store makes and the user does not need.
///
/// A code that is not in this list keeps its own number. Guessing at an
/// unknown state would be worse than showing it: at least a number can be
/// looked up.
pub(crate) fn status_name(code: &str) -> String {
    match code {
        "0" => "unlicensed",
        "1" => "licensed",
        "2" | "3" | "6" => "grace",
        "4" => "non_genuine",
        "5" => "notification",
        other => return other.to_string(),
    }
    .to_string()
}

/// The names [`status_name`] can produce, for deciding whether a status has a
/// translation or is a raw code being passed through.
pub(crate) const STATUS_NAMES: [&str; 5] = [
    "licensed",
    "unlicensed",
    "grace",
    "notification",
    "non_genuine",
];

/// Windows first, then Office, then anything else; within each family the ones
/// that are **not** licensed come first, because they are why somebody opened
/// this. Ties break by name so the order does not wander between scans.
pub(crate) fn sort_licenses(lics: &mut [License]) {
    lics.sort_by(|a, b| {
        let family = |l: &License| match l.kind.as_str() {
            "windows" => 0,
            "office" => 1,
            _ => 2,
        };
        family(a)
            .cmp(&family(b))
            .then(a.ok().cmp(&b.ok()))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}
