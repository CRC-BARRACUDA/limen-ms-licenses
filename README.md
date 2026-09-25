# limen-ms-licenses

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

The **OEM key in firmware** is the one exception, and it is a deliberate one: it
is shown in full, because recovering the key of the machine you are sitting at is
the entire reason anybody looks for it, and a masked one would be useless. It is
never read unless you press the button and then tell Windows to allow it.

## What it asks for

`subprocess`, to run PowerShell, and `elevate` — for one field, when asked.

- **No `network`.** Activation state is read locally. The module does not phone
  Microsoft and does not report anywhere.
- **`elevate`, for one field and on request.** The OEM product key held in
  firmware needs administrator on most builds. Everything else on the screen is
  read as an ordinary user, with no prompt at all; that one field comes back
  marked unreadable, with a button beside it. Pressing it runs **one command** —
  a single property, written to a file in this module's own directory — and
  Windows asks you whether to allow it. Nothing else is elevated, nothing is
  elevated without being asked for, and refusing costs you only that field.
- **Nothing is changed.** No activation, no rearm, no install, no removal. The
  only file written is the one the elevated read uses to hand the value back,
  and it is deleted as soon as it has been read.

## Why it refuses to run elsewhere

The manifest says `os = ["windows"]`, and Limen honours it: a module that names
the platforms it runs on, on one it did not name, is dropped at discovery. It is
not started, not listed in the module manager, and not reported as broken —
because nothing is broken. It is for another machine.

Without that, this module installs on Linux, starts, finds no licensing store, and
answers with an empty list. **An empty list reads like a finding**: nothing on
this machine is licensed. "This module is for windows" reads like what it is.

## For other modules

It provides `licenses.ms`:

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

## Building and testing

A native (`cdylib`) module, written in Rust against `limen-sdk-rust`.

```sh
cargo test                      # runs on any platform
scripts/package-windows.ps1     # the release asset + its .sha256
```

The tests run anywhere, including on Linux. The PowerShell query is Windows only
— which is the point of the module — so what they cover is everything around it:
the parsing of what it prints, the order rows come out in, the screens in both
languages, what the report provider is handed, and that every string the code
asks for exists in both catalogs with nothing unused. The query itself is
verified on a Windows host.

There is no `package.sh`: a `.so` built from these sources is an artifact no
machine could load.
