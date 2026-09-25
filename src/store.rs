//! Reading the licensing store, and making sense of what it prints.
//!
//! One PowerShell run, printing one record per line:
//!
//! ```text
//! lic|kind|name|status|channel|key|expires|kms|description
//! fact|section|item|value
//! ```
//!
//! A flat line format rather than JSON, deliberately: a truncated list of lines
//! is worth all but its last line, and a truncated JSON document is worth
//! nothing. The query separates its own fields with `|` and strips that
//! character out of every value before printing, so a value can never split a
//! record.

use crate::*;

/// The two application ids that tell Windows and Office apart inside the one
/// class. Anything else is reported under its own name rather than guessed at.
const WINDOWS_APP_ID: &str = "55c92734-d682-4d71-983e-d6ec3f16059f";
const OFFICE_APP_ID: &str = "0ff1ce15-a989-479d-af46-f275c6370663";

/// What the query said about its own run, as opposed to what it found.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Flags {
    /// The query started. Its absence means PowerShell never got going — and an
    /// empty list then means "not read", not "nothing licensed".
    pub ran: bool,
    /// The OEM key held in firmware could not be read (administrator on some
    /// builds). Reported, rather than left as a blank that reads like "no key".
    pub no_oem: bool,
    /// The query itself raised something, carried out rather than swallowed.
    pub error: String,
}

/// The query, with `WINDOWS_APP_ID` / `OFFICE_APP_ID` still to be spliced in by
/// [`query`] — so the two ids are written in exactly one place.
///
/// `SoftwareLicensingProduct` carries both Windows and Office on a modern
/// system; Office 2010-2013 predates that and keeps its own class. Click-to-Run
/// Office has no product key to speak of, so its channel and build come from
/// the registry — for a subscription install that is the only thing
/// identifying it.
const QUERY: &str = r#"
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
# The filter belongs in the query, not in a loop over the answer.
#
# `SoftwareLicensingProduct` lists every edition Windows *could* be — 79 of them
# on an ordinary machine, of which two have a key installed. Each instance is
# materialised by the Software Protection Platform, which is not cheap, so
# enumerating the class costs the same whether the caller wanted all of it or
# not: measured at 11 seconds here. The same query with the WHERE clause below
# takes 111 milliseconds and returns byte-identical records, because the
# provider evaluates it before doing that work.
#
# Naming the columns rather than SELECT * is the rest of it — four times faster
# again, for the same reason.
$COLS = 'Name,LicenseStatus,ProductKeyChannel,PartialProductKey,GracePeriodRemaining,' +
        'KeyManagementServiceMachine,KeyManagementServicePort,' +
        'DiscoveredKeyManagementServiceMachineName,Description,ApplicationID'
try {
foreach ($p in @(Get-CimInstance -Query ("SELECT $COLS FROM SoftwareLicensingProduct WHERE PartialProductKey IS NOT NULL"))) {
  $app = ([string]$p.ApplicationID).ToLower()
  $kind = 'other'
  if ($app -eq '__WINDOWS_APP_ID__') { $kind = 'windows' }
  elseif ($app -eq '__OFFICE_APP_ID__') { $kind = 'office' }
  Product $kind $p
}
# Office 2010-2013 predates the shared class and keeps its own.
foreach ($p in @(Get-CimInstance -Query ("SELECT $COLS FROM OfficeSoftwareProtectionProduct WHERE PartialProductKey IS NOT NULL"))) { Product 'office' $p }

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
"#;

/// The query as it is actually run.
pub(crate) fn query() -> String {
    QUERY
        .replace("__WINDOWS_APP_ID__", WINDOWS_APP_ID)
        .replace("__OFFICE_APP_ID__", OFFICE_APP_ID)
}

/// The file an elevated run leaves the OEM key in.
///
/// A file rather than a pipe, because an elevated command's output does not
/// come back: the host reports whether it ran, not what it printed. It is
/// written inside the module's own directory rather than the system temp dir —
/// that is under Limen's base, where this module is the only thing with a
/// reason to write, whereas everything on the machine can write to temp and
/// this value is a product key.
pub(crate) fn oem_file(module_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(module_dir).join(".oem-key")
}

/// The command that reads the OEM key, for the host to run elevated.
///
/// One property, written to `file` and nothing else. Deliberately not the whole
/// query again: what needs administrator is this single field, so this is all
/// that is asked to run with privileges.
///
/// Absolute paths throughout — elevation replaces the environment with a
/// minimal one, so nothing resolved through `PATH` would be found.
pub(crate) fn oem_argv(file: &std::path::Path) -> Vec<String> {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let shell = format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe");
    // The path goes inside a single-quoted PowerShell string, so a quote in it
    // has to be doubled or it would end the literal.
    let quoted = file.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue';\
         $k=(Get-CimInstance SoftwareLicensingService).OA3xOriginalProductKey;\
         Set-Content -LiteralPath '{quoted}' -Value ([string]$k).Trim() -Encoding ASCII -NoNewline"
    );
    vec![
        shell,
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        script,
    ]
}

/// Read back what the elevated run left, and take the file away.
///
/// Removed whether or not it held anything: the value is a product key, and
/// leaving it on disk after it has been shown once would be a copy nobody
/// remembers making. `None` when the run wrote nothing — a machine with no OEM
/// key in firmware at all, which is an ordinary thing for a retail install.
pub(crate) fn take_oem(file: &std::path::Path) -> Option<String> {
    let key = std::fs::read_to_string(file).ok();
    let _ = std::fs::remove_file(file);
    let key = key?.trim().to_string();
    (!key.is_empty()).then_some(key)
}

/// Why a scan produced nothing.
pub(crate) enum ReadError {
    /// PowerShell could not be started at all.
    NoPowerShell,
    /// Anything else, carried as it was reported.
    Other(String),
}

/// Run the query and hand back its stdout.
///
/// The window is suppressed: this is a GUI module, and a console flashing up on
/// every scan is noise the user did not ask for.
#[cfg(target_os = "windows")]
pub(crate) fn read_store() -> Result<String, ReadError> {
    use limen_proto::NoConsole;
    // The absolute path rather than `powershell`: a `powershell.exe` earlier on
    // PATH would otherwise be what reads this machine's licensing state.
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let shell = format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe");
    let out = std::process::Command::new(shell)
        .args(["-NoProfile", "-NonInteractive", "-Command", &query()])
        .no_console()
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ReadError::NoPowerShell,
            _ => ReadError::Other(e.to_string()),
        })?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Off Windows there is no licensing store to read. The module is not offered
/// on such a machine at all — its manifest says `os = ["windows"]` and the host
/// drops it at discovery — so this exists to keep the crate building and
/// testable everywhere, which is where the parser is exercised.
#[cfg(not(target_os = "windows"))]
pub(crate) fn read_store() -> Result<String, ReadError> {
    Err(ReadError::NoPowerShell)
}

/// The query's output into licences, facts, and what its markers said.
pub(crate) fn parse(stdout: &str) -> (Vec<License>, Vec<Fact>, Flags) {
    let mut lics = Vec::new();
    let mut facts = Vec::new();
    let mut flags = Flags::default();

    for line in stdout.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        // Markers: what the run did, as opposed to what it found.
        if let Some(rest) = line.strip_prefix('#') {
            match rest {
                "begin" => flags.ran = true,
                "no-oem" => flags.no_oem = true,
                _ => {
                    if let Some(msg) = rest.strip_prefix("err=") {
                        flags.error = msg.trim().to_string();
                    }
                }
            }
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        match parts[0] {
            // The tail is joined back up: `Clean` strips `|` out of every field
            // before printing, so this is belt-and-braces rather than
            // load-bearing — but a value that somehow kept one should arrive
            // whole rather than truncated at it.
            "fact" if parts.len() >= 4 => facts.push(Fact {
                section: parts[1].to_string(),
                item: parts[2].to_string(),
                value: parts[3..].join("|"),
            }),
            "lic" if parts.len() >= 9 => lics.push(License {
                kind: parts[1].to_string(),
                name: parts[2].to_string(),
                status: status_name(parts[3]),
                channel: parts[4].to_string(),
                key: parts[5].to_string(),
                expires: parts[6].to_string(),
                kms: parts[7].to_string(),
                description: parts[8..].join("|"),
            }),
            // Anything else is a line PowerShell wrote that is not a record —
            // skipped rather than guessed at.
            _ => {}
        }
    }
    sort_licenses(&mut lics);
    (lics, facts, flags)
}
