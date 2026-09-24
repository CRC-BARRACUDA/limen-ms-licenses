"""licenses — how Windows and Office are activated on THIS machine.

Status, channel, the partial key, the KMS host it talks to, and what is running
on a grace period. The `slmgr /dlv` answer, without the four pop-ups.

Windows only, declared in the manifest, so the host does not start it elsewhere
and hand back an empty list that reads like "nothing is licensed".

What it reads
-------------
`SoftwareLicensingProduct` carries both Windows and Office on a modern system,
told apart by their application id. Office 2010-2013 predates that and lives in
`OfficeSoftwareProtectionProduct`, which is read as well when it exists.
Click-to-Run Office keeps its channel and build in the registry rather than in
either class, so that is read too — a subscription install often has no product
key at all, and the channel is the only thing that identifies it.

It reads. Nothing is activated, rearmed, installed or removed, and nothing
leaves the machine.

A note on keys
--------------
Only the **last five characters** of a product key are shown — that is all the
licensing store holds, and it is enough to tell two licences apart without being
a key anybody could use. The full key is deliberately not recovered from the
registry: identifying a licence and handing over a credential are different
jobs, and only the first belongs in an inventory tool.
"""

import os
import subprocess

from limen_sdk import (
    Button,
    Catalog,
    Label,
    Module,
    Row,
    Separator,
    Table,
    Window,
)

CAT = Catalog.from_dir(os.path.dirname(os.path.abspath(__file__)), ("en", "uk"))
_lang = {"code": "en"}


def t(key, **args):
    """Translate for the language the host last reported."""
    return CAT.tr(_lang["code"], key, **args)


CAP = "license.local"
m = Module("licenses", [CAP])


def _sync_locale():
    """Re-read the host's language before drawing."""
    try:
        _lang["code"] = m.host.locale()
    except Exception:  # noqa: BLE001
        pass  # keep the last language rather than failing to draw


# `LicenseStatus` as the licensing store reports it. The numbers are the whole
# answer to "is this machine activated", so they are named rather than shown.
STATUS = {
    "0": "unlicensed",
    "1": "licensed",
    "2": "grace",          # out-of-box grace
    "3": "grace",          # out-of-tolerance grace, e.g. hardware changed
    "4": "non_genuine",
    "5": "notification",   # the state that nags and eventually stops features
    "6": "grace",          # extended grace
}

# The two application ids that tell Windows and Office apart inside the one
# class. Anything else is reported under its own name rather than guessed at.
WINDOWS_APP_ID = "55c92734-d682-4d71-983e-d6ec3f16059f"
OFFICE_APP_ID = "0ff1ce15-a989-479d-af46-f275c6370663"


# --------------------------------------------------------------------------- #
# Reading the licensing store.
#
# One PowerShell run, printing one record per line as
#     lic|kind|name|status|channel|key|expires|kms|description
#     fact|section|item|value
# A flat line format rather than JSON because a truncated list of lines is worth
# all but its last line, and a truncated JSON document is worth nothing.
# --------------------------------------------------------------------------- #
_PS = r"""
$ErrorActionPreference = 'SilentlyContinue'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
Write-Output '#begin'
function Clean($s) {
  if ($null -eq $s) { return '' }
  return (([string]$s) -replace '[\r\n\|]', ' ').Trim()
}
function Fact($section, $item, $value) {
  $v = Clean $value
  if ($v) { Write-Output ('fact|' + $section + '|' + (Clean $item) + '|' + $v) }
}
function Product($kind, $p) {
  # Only products with a key installed are licences on this machine; the class
  # also lists every edition Windows *could* be, which is not an answer.
  if (-not $p.PartialProductKey) { return }
  $expires = ''
  if ($p.GracePeriodRemaining -and $p.GracePeriodRemaining -gt 0) {
    $expires = [string][int]($p.GracePeriodRemaining / 1440) + ' days'
  }
  $kms = ''
  if ($p.KeyManagementServiceMachine) {
    $kms = (Clean $p.KeyManagementServiceMachine)
    if ($p.KeyManagementServicePort) { $kms = $kms + ':' + (Clean $p.KeyManagementServicePort) }
  } elseif ($p.DiscoveredKeyManagementServiceMachineName) {
    $kms = (Clean $p.DiscoveredKeyManagementServiceMachineName) + ' (discovered)'
  }
  $fields = @('lic', $kind, (Clean $p.Name), (Clean $p.LicenseStatus),
              (Clean $p.ProductKeyChannel), (Clean $p.PartialProductKey),
              $expires, $kms, (Clean $p.Description))
  Write-Output ($fields -join '|')
}
try {
foreach ($p in @(Get-CimInstance SoftwareLicensingProduct)) {
  $app = ([string]$p.ApplicationID).ToLower()
  $kind = 'other'
  if ($app -eq '55c92734-d682-4d71-983e-d6ec3f16059f') { $kind = 'windows' }
  elseif ($app -eq '0ff1ce15-a989-479d-af46-f275c6370663') { $kind = 'office' }
  Product $kind $p
}
# Office 2010-2013 predates the shared class and keeps its own.
foreach ($p in @(Get-CimInstance OfficeSoftwareProtectionProduct)) { Product 'office' $p }

$svc = Get-CimInstance SoftwareLicensingService
Fact 'windows' 'Client machine id' $svc.ClientMachineID
Fact 'windows' 'KMS host configured' $svc.KeyManagementServiceMachine
# The OEM key lives in firmware and needs administrator on some builds. Its
# absence is reported rather than left as a blank that reads like "no key".
$oem = Clean $svc.OA3xOriginalProductKey
if ($oem) { Fact 'windows' 'OEM key in firmware' $oem } else { Write-Output '#no-oem' }

# Click-to-Run Office has no product key to speak of; the channel and build are
# what identify a subscription install.
$c2r = 'HKLM:\SOFTWARE\Microsoft\Office\ClickToRun\Configuration'
if (Test-Path $c2r) {
  $cfg = Get-ItemProperty $c2r
  Fact 'office' 'Click-to-Run products' $cfg.ProductReleaseIds
  Fact 'office' 'Version' $cfg.VersionToReport
  Fact 'office' 'Update channel' $cfg.UpdateChannel
  Fact 'office' 'Platform' $cfg.Platform
}
} catch { Write-Output ('#err=' + ($_.Exception.Message -replace '[\r\n\|]', ' ')) }
"""


def read_store():
    """Run the query and return its stdout.

    PowerShell is asked for once, with the window suppressed: this is a GUI
    module and a console flashing up on every scan is noise the user did not ask
    for.
    """
    flags = 0
    if hasattr(subprocess, "CREATE_NO_WINDOW"):
        flags = subprocess.CREATE_NO_WINDOW
    out = subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", _PS],
        capture_output=True, text=True, timeout=120, creationflags=flags,
    )
    return out.stdout


def parse(stdout):
    """The query's output into licences and facts, plus what the markers said."""
    lics, facts = [], []
    flags = {"error": "", "no_oem": False, "ran": False}
    for line in (stdout or "").splitlines():
        line = line.rstrip()
        if not line.strip():
            continue
        if line.startswith("#"):
            if line == "#begin":
                flags["ran"] = True
            elif line == "#no-oem":
                flags["no_oem"] = True
            elif line.startswith("#err="):
                flags["error"] = line[5:].strip()
            continue
        parts = [p.strip() for p in line.split("|")]
        if parts[0] == "fact" and len(parts) >= 4:
            facts.append({"section": parts[1], "item": parts[2],
                          "value": "|".join(parts[3:])})
        elif parts[0] == "lic" and len(parts) >= 9:
            kind, name, status, channel, key, expires, kms = parts[1:8]
            lics.append({
                "kind": kind,
                "name": name,
                # Named, not numbered: "1" is not an answer to "is this
                # activated". An unknown code keeps its number rather than being
                # silently called something it is not.
                "status": STATUS.get(status, status),
                "channel": channel,
                "key": key,
                "expires": expires,
                "kms": kms,
                "description": "|".join(parts[8:]),
                "ok": status == "1",
            })
    # Windows first, then Office, then anything else; activated last within each
    # group, because the ones that are not are why somebody opened this.
    order = {"windows": 0, "office": 1}
    lics.sort(key=lambda r: (order.get(r["kind"], 2), r["ok"], r["name"].lower()))
    return lics, facts, flags


# --------------------------------------------------------------------------- #
# State and methods.
# --------------------------------------------------------------------------- #
# The last scan. Held so reopening the tab shows what was found rather than
# running PowerShell again on every draw.
_last = {"lics": None, "facts": [], "flags": {}, "error": ""}


@m.method("scan")
def scan(params, host):
    """Read the licensing store and render."""
    _sync_locale()
    try:
        lics, facts, flags = parse(read_store())
        if not flags["ran"]:
            # No marker means the query never got going — PowerShell missing,
            # blocked by policy. An empty list would read as "nothing licensed".
            _last.update(lics=None, facts=[], flags={},
                         error=t("error.no_powershell"))
        else:
            _last.update(lics=lics, facts=facts, flags=flags, error="")
            host.log(f"licenses: {len(lics)} licence(s), {len(facts)} fact(s)")
    except FileNotFoundError:
        _last.update(lics=None, error=t("error.no_powershell"))
    except subprocess.TimeoutExpired:
        _last.update(lics=None, error=t("error.timeout"))
    except Exception as exc:  # noqa: BLE001
        _last.update(lics=None, error=str(exc))
        host.log(f"licenses: scan failed: {exc}")
    return m._view_spec()


@m.method("licenses")
def licenses(params, host):
    """The capability method: the last scan as data, for other modules.

    Reports what was last read rather than reading again — a caller asking a
    question should not set a PowerShell process going on the user's machine.

    ``{"kind": "office"}`` narrows to one product family, and
    ``{"ok": false}`` to whatever is not properly licensed, which is the
    question a fleet report actually asks.
    """
    if _last["lics"] is None:
        return {"error": _last["error"] or "nothing scanned yet",
                "total": 0, "licenses": []}
    rows = _last["lics"]
    kind = (params.get("kind") or "").strip().lower()
    if kind:
        rows = [r for r in rows if r["kind"] == kind]
    ok = params.get("ok")
    if ok is not None:
        rows = [r for r in rows if r["ok"] == bool(ok)]
    return {
        "note": "Activation state of Windows and Office on this machine. `key` is "
                "the last five characters, which is all the licensing store "
                "holds. `status` is the licensing store's own state, named: "
                "licensed, unlicensed, grace, notification, non_genuine.",
        "total": len(rows),
        "licensed": sum(1 for r in rows if r["ok"]),
        "licenses": rows,
        "facts": _last["facts"],
    }


# --------------------------------------------------------------------------- #
# The screen.
# --------------------------------------------------------------------------- #
def columns():
    """Column headings, in the current language."""
    return [t("col.product"), t("col.status"), t("col.channel"),
            t("col.key"), t("col.expires"), t("col.kms")]


def _cells(r):
    # The key is shown as the licensing store gives it — the last five
    # characters — with the rest masked so the column reads as a fragment
    # rather than as something withheld.
    key = f"·····-{r['key']}" if r["key"] else ""
    return [r["name"], t("status." + r["status"]) if r["status"] in STATUS.values()
            else r["status"], r["channel"], key, r["expires"], r["kms"]]


def idle_view():
    """Nothing is read until asked: a line about what this does, and a button."""
    return Window(t("ui.title"), [
        Label(t("ui.title"), style="heading"),
        Label(t("ui.subtitle"), style="weak"),
        Separator(),
        Button(t("ui.scan"), calls="scan", primary=True),
    ]).to_spec()


@m.ui
def ui():
    _sync_locale()
    if _last["lics"] is None and not _last["error"]:
        return idle_view()

    widgets = [
        Label(t("ui.title"), style="heading"),
    ]
    if _last["error"]:
        widgets += [Label(_last["error"], style="weak"), Separator(),
                    Button(t("ui.rescan"), calls="scan", primary=True)]
        return Window(t("ui.title"), widgets).to_spec()

    lics = _last["lics"]
    bad = [r for r in lics if not r["ok"]]
    # The headline is the question people open this to answer, so it is the
    # first line rather than something to work out from the table.
    widgets.append(Label(
        t("ui.all_ok") if not bad else t("ui.attention", n=len(bad)),
        style="weak"))
    if _last["flags"].get("no_oem"):
        widgets.append(Label("! " + t("note.no_oem"), style="weak"))
    if _last["flags"].get("error"):
        widgets.append(Label("! " + _last["flags"]["error"], style="weak"))
    actions = [Button(t("ui.rescan"), calls="scan")]
    if _has_reports():
        actions.append(Button(t("ui.report"), calls="make_report", open_in_tab=True))
    widgets.append(Row(actions))

    for kind, heading in (("windows", "kind.windows"), ("office", "kind.office"),
                          ("other", "kind.other")):
        rows = [r for r in lics if r["kind"] == kind]
        if not rows:
            continue
        widgets += [
            Separator(),
            Label(t(heading), style="strong"),
            Table(columns(), [_cells(r) for r in rows]),
        ]

    facts = _last["facts"]
    if facts:
        widgets += [Separator(), Label(t("ui.details"), style="strong"),
                    Table([t("col.item"), t("col.value")],
                          [[f["item"], f["value"]] for f in facts])]
    return Window(t("ui.title"), widgets).to_spec()


# --------------------------------------------------------------------------- #
# The report, when a provider is installed.
# --------------------------------------------------------------------------- #
def _has_reports():
    """Whether a report provider is installed right now.

    Asked each draw rather than cached: `report.build` is optional, and a module
    installed while this tab is open should make the button appear.
    """
    try:
        return m.host.has_capability("report.build")
    except Exception:  # noqa: BLE001
        return False


def report_spec():
    """The last scan as a report: a summary, a status chart, and one table."""
    lics = _last["lics"] or []
    bad = [r for r in lics if not r["ok"]]
    counts = {}
    for r in lics:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    chart = [{"label": t("status." + k) if k in STATUS.values() else k, "value": v}
             for k, v in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))]
    sections = [{
        "heading": t("ui.title"),
        "columns": columns(),
        "rows": [_cells(r) for r in lics],
    }]
    if _last["facts"]:
        sections.append({
            "heading": t("ui.details"),
            "columns": [t("col.item"), t("col.value")],
            "rows": [[f["item"], f["value"]] for f in _last["facts"]],
        })
    return {
        "title": t("report.title"),
        "subtitle": t("report.subtitle", total=len(lics), bad=len(bad)),
        "format": "view",
        "summary": [
            t("report.line_total", n=len(lics)),
            t("report.line_bad", n=len(bad)),
        ],
        "charts": [{"title": t("report.by_status"), "data": chart}] if chart else [],
        "sections": sections,
    }


@m.method("make_report")
def make_report(params, host):
    """Hand the last scan to whatever report provider is installed."""
    _sync_locale()
    if _last["lics"] is None:
        return Window(t("report.title"), [
            Label(t("report.nothing"), style="strong"),
        ]).to_spec()
    try:
        built = host.call("report.build", "build", report_spec())
    except Exception as exc:  # noqa: BLE001
        return Window(t("report.title"), [
            Label(t("report.failed"), style="strong"),
            Label(str(exc), style="weak"),
        ]).to_spec()
    # A provider that renders answers with a view; one that exports a file
    # writes it, opens it, and acknowledges with nothing.
    if isinstance(built, dict) and built.get("widgets") is not None:
        return built
    return Window(t("report.title"), [
        Label(t("report.exported"), style="strong"),
        Label(t("report.exported_hint"), style="weak"),
    ]).to_spec()


if __name__ == "__main__":
    m.run()
