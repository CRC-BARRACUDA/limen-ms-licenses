# limen-licenses

How **Windows and Office are activated on this machine** — status, channel, the
partial key, the KMS host, and whatever is running on a grace period. The
`slmgr /dlv` answer, without the four pop-ups, and Office alongside Windows
rather than in a second place.

Windows only, and it says so. See [Why it refuses to run elsewhere](#why-it-refuses-to-run-elsewhere).

## What it reads

| Source | What it answers |
|---|---|
| `SoftwareLicensingProduct` | Windows and modern Office: status, channel, partial key, grace, KMS host |
| `OfficeSoftwareProtectionProduct` | Office 2010–2013, which predates the shared class |
| `SoftwareLicensingService` | client machine id, the configured KMS host, the OEM key in firmware |
| `HKLM\SOFTWARE\Microsoft\Office\ClickToRun\Configuration` | a subscription install's products, build and update channel |

Click-to-Run Office often has **no product key at all**, so the channel and build
are the only things that identify it. Reading only the licensing classes would
show a Microsoft 365 machine as having no Office on it.

Products with no key installed are dropped: the class also lists every edition
Windows *could* be, and a list of hypothetical editions is not an answer.

## Status is named, not numbered

`LicenseStatus` is the whole answer to "is this activated", and it arrives as a
number. `1` is not an answer, so each is named — licensed, not licensed, grace
period, notification, not genuine — and the three separate grace codes
(out-of-box, out-of-tolerance, extended) are all "grace period", which is what
the person asking wants to know. A code Microsoft adds later keeps its number
rather than being silently called something it is not.

Anything not properly licensed sorts to the top of its group, because it is why
somebody opened this.

## Keys

Only the **last five characters** of a key are shown — that is all the licensing
store holds — and the column masks the rest so it reads as a fragment rather than
as something withheld.

The full key is deliberately **not** recovered from the registry, which is a thing
this module could do and does not. Identifying a licence and handing over a
credential are different jobs, and only the first belongs in an inventory tool.

## What it asks for

`subprocess`, to run PowerShell. That is the lot.

- **No `network`.** Activation state is read locally. The module does not phone
  Microsoft and does not report anywhere.
- **No `elevate`.** One field — the OEM key held in firmware — needs
  administrator on some builds. Rather than ask for elevation for one field, the
  module reports that field as unreadable and says why. Everything else works as
  an ordinary user.
- **Nothing is changed.** No activation, no rearm, no install, no removal, no
  files written. The query reads.

## Why it refuses to run elsewhere

The manifest says `os = ["windows"]`, and Limen does not start the module on
anything else — it stays listed, with the reason on the card.

Without that, this module installs on Linux, starts, finds no licensing store, and
answers with an empty list. **An empty list reads like a finding**: nothing on
this machine is licensed. "This module is for windows" reads like what it is.

## For other modules

It provides `license.local`:

```json
{ "method": "licenses", "params": { "kind": "office", "ok": false } }
```

`kind` narrows to `windows`, `office` or `other`; `ok: false` returns whatever is
not properly licensed, which is the question a fleet report actually asks. It
answers from the **last scan** rather than scanning again — a caller asking a
question should not set a PowerShell process going on somebody's machine.

If a `report.build` provider is installed, a **Make report** button appears, with
a status chart and the tables.

## Languages

English and Ukrainian, screens and all — not just the module card. The host says
which, and the screen follows on the next draw.

## Tests

```sh
python3 tests.py
```

Runs anywhere, including here. The PowerShell query is Windows only — which is the
point of the module — so what the tests cover is everything around it: the parsing
of what it prints, the order rows come out in, the screens in both languages, the
capability method's filtering, and that every string the code asks for exists in
both catalogs with nothing unused. The query itself is verified on a Windows host.
