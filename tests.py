"""What this module is expected to do, in the language of what it is for.

Run it anywhere:

    python3 tests.py

The PowerShell query itself cannot run here — it is Windows only, which is the
whole point of the module — so what is tested is everything around it: the
parsing of what it prints, the order things come out in, the screens, and that
every string the code asks for exists in both languages.
"""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                "..", "..", "sdk", "python"))
import main as mod  # noqa: E402


class FakeHost:
    """The host, without the pipe.

    `main` asks the host two things while drawing — which language, and whether a
    report provider is installed — and both go down the JSON-RPC pipe to a host
    that is not there. Standing in for it is what makes the screens testable at
    all.
    """

    def __init__(self, lang="en", caps=()):
        self.lang, self.caps, self.logs = lang, list(caps), []

    def locale(self):
        return self.lang

    def capabilities(self):
        return self.caps

    def has_capability(self, cap):
        return cap in self.caps

    def log(self, message):
        self.logs.append(message)

    def call(self, capability, method, params=None):
        raise RuntimeError("no provider")


mod.m.host = FakeHost()


# A real-shaped answer: Windows activated by a KMS host, Office in its grace
# period with no key, and a Click-to-Run install that has no licence row at all.
SAMPLE = "\n".join([
    "#begin",
    "lic|windows|Windows(R), Professional edition|1|Volume:GVLK|3V66T||kms.corp.test:1688|Windows Pro",
    "lic|office|Office 16, Office16ProPlusVL_KMS_Client edition|2|Volume:GVLK|WFG99|18 days||Office",
    "lic|other|Windows(R), ServerStandard edition|0|Retail|Q9MRV|||Server",
    "fact|windows|Client machine id|abcd-1234",
    "fact|office|Update channel|Current",
    "#no-oem",
])


def check(name, cond, detail=""):
    if cond:
        print(f"  ok   {name}")
        return 0
    print(f"  FAIL {name} {detail}")
    return 1


def test_parse():
    """The lines come back as licences, facts and markers."""
    bad = 0
    lics, facts, flags = mod.parse(SAMPLE)
    bad += check("three licences", len(lics) == 3, lics)
    bad += check("two facts", len(facts) == 2, facts)
    bad += check("the run is marked as having started", flags["ran"])
    bad += check("the missing OEM key is reported", flags["no_oem"])
    bad += check("no error", flags["error"] == "")

    # Numbers are not an answer to "is this activated"; names are.
    by_kind = {r["kind"]: r for r in lics}
    bad += check("1 is licensed", by_kind["windows"]["status"] == "licensed")
    bad += check("2 is a grace period", by_kind["office"]["status"] == "grace")
    bad += check("0 is unlicensed", by_kind["other"]["status"] == "unlicensed")
    bad += check("only status 1 counts as ok",
                 [r["ok"] for r in lics].count(True) == 1)

    bad += check("the KMS host survives its port",
                 by_kind["windows"]["kms"] == "kms.corp.test:1688")
    bad += check("the grace period is carried",
                 by_kind["office"]["expires"] == "18 days")
    bad += check("only the partial key is ever present",
                 all(len(r["key"]) <= 5 for r in lics), [r["key"] for r in lics])
    return bad


def test_order():
    """Windows first, then Office, and within each the ones that are wrong."""
    rows = "\n".join([
        "#begin",
        "lic|office|Office B|1|Retail|AAAAA|||",
        "lic|office|Office A|0|Retail|BBBBB|||",
        "lic|windows|Windows|1|Retail|CCCCC|||",
        "lic|other|Something|1|Retail|DDDDD|||",
    ])
    lics, _, _ = mod.parse(rows)
    order = [(r["kind"], r["name"]) for r in lics]
    return check("windows, then office, then the rest — unlicensed first",
                 order == [("windows", "Windows"), ("office", "Office A"),
                           ("office", "Office B"), ("other", "Something")], order)


def test_markers():
    """A query that never got going is not an empty list of licences."""
    bad = 0
    _, _, flags = mod.parse("")
    bad += check("nothing at all is not a successful run", not flags["ran"])
    _, _, flags = mod.parse("#begin\n#err=Access is denied.")
    bad += check("an error is carried out of PowerShell",
                 flags["error"] == "Access is denied.", flags)

    # A product with no key installed is every edition Windows *could* be, and
    # the query drops it. The parser must not invent one either.
    lics, _, _ = mod.parse("#begin\nlic|windows|W|1|Retail||||")
    bad += check("a row with no key still parses", len(lics) == 1)
    return bad


def test_unknown_status_keeps_its_number():
    """A status the licensing store grows later is not silently renamed."""
    lics, _, _ = mod.parse("#begin\nlic|windows|W|9|Retail|AAAAA|||")
    return check("an unknown code is shown as itself",
                 lics[0]["status"] == "9", lics[0])


def test_pipe_in_a_value():
    """A pipe inside a value would split a record; the query strips them."""
    return check("the query cleans separators out of every field",
                 "-replace '[\\r\\n\\|]'" in mod._PS)


def test_views():
    """Every screen is a view the host can actually deserialize."""
    bad = 0
    mod.m.host = FakeHost()
    mod._last.update(lics=None, facts=[], flags={}, error="")
    idle = mod.idle_view()
    bad += check("the idle screen is a window with a title",
                 idle.get("title") and isinstance(idle.get("widgets"), list), idle)
    bad += check("nothing is read until asked",
                 any(w.get("kind") == "button" for w in idle["widgets"]))

    lics, facts, flags = mod.parse(SAMPLE)
    mod._last.update(lics=lics, facts=facts, flags=flags, error="")
    mod.m._own_capability()
    view = mod.m._ui()
    kinds = [w["kind"] for w in view["widgets"]]
    bad += check("the scan screen has a table per product family",
                 kinds.count("table") == 4, kinds)  # windows, office, other, facts
    text = json.dumps(view, ensure_ascii=False)
    bad += check("the headline counts what needs attention", "2 product" in text, text[:400])
    bad += check("the missing OEM key is said out loud", "OEM" in text)

    # Nothing anywhere may print a whole product key.
    bad += check("the key column is masked", "·····-3V66T" in text)

    # The report button is the optional integration: absent unless a provider is.
    bad += check("no report button without a provider", "make_report" not in text)
    mod.m.host = FakeHost(caps=["report.build"])
    with_report = json.dumps(mod.m._ui(), ensure_ascii=False)
    bad += check("and one when there is", "make_report" in with_report)
    mod.m.host = FakeHost()

    mod._last.update(lics=None, error="boom")
    err = mod.m._ui()
    bad += check("a failure offers to try again",
                 any(w.get("kind") == "button" for w in err["widgets"]), err)
    return bad


def test_capability_method():
    """The capability answers with data, filtered the way a caller asks."""
    bad = 0
    lics, facts, flags = mod.parse(SAMPLE)
    mod._last.update(lics=lics, facts=facts, flags=flags, error="")

    every = mod.licenses({}, None)
    bad += check("everything, counted", every["total"] == 3 and every["licensed"] == 1, every)
    bad += check("the facts come with it", len(every["facts"]) == 2)

    office = mod.licenses({"kind": "OFFICE"}, None)
    bad += check("one family", office["total"] == 1, office)

    wrong = mod.licenses({"ok": False}, None)
    bad += check("what is not properly licensed — the question a fleet asks",
                 wrong["total"] == 2, wrong)

    mod._last.update(lics=None, error="")
    empty = mod.licenses({}, None)
    bad += check("nothing scanned is an error, not an empty inventory",
                 empty["error"] and empty["total"] == 0, empty)
    return bad


def test_translations():
    """Every key the code asks for exists in both languages."""
    import re
    src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "main.py"),
               encoding="utf-8").read()
    asked = set(re.findall(r't\(\s*"([a-z_]+\.[a-z_]+)"', src))
    # `t("status." + r["status"])` is built at runtime from the status table.
    asked |= {"status." + v for v in set(mod.STATUS.values())}
    # And the per-family headings, which `ui` looks up by a key it carries in a
    # table rather than writing out at the call.
    asked |= {"kind.windows", "kind.office", "kind.other"}
    bad = check("the code asks for a good few strings", len(asked) > 20, len(asked))
    for lang in ("en", "uk"):
        table = mod.CAT._langs[lang]
        missing = sorted(k for k in asked if k not in table)
        bad += check(f"{lang} has every key", not missing, missing)
        # And nothing sitting in the file that nothing asks for.
        spare = sorted(k for k in table if k not in asked and not k.startswith("module."))
        bad += check(f"{lang} carries nothing unused", not spare, spare)
    return bad


def test_the_screen_follows_the_host_language():
    """The host says Ukrainian; the screen is in Ukrainian, headings and all."""
    lics, facts, flags = mod.parse(SAMPLE)
    mod._last.update(lics=lics, facts=facts, flags=flags, error="")
    mod.m.host = FakeHost(lang="uk_UA")
    mod.m._own_capability()
    text = json.dumps(mod.m._ui(), ensure_ascii=False)
    bad = check("the title is translated", "Ліцензії Windows та Office" in text)
    bad += check("so is the table's own content — a status, not just a heading",
                 "Пільговий період" in text, text[:600])
    bad += check("and nothing English is left in the chrome",
                 "Check again" not in text)
    mod.m.host = FakeHost()
    return bad


def test_placeholders_survive_translation():
    """A translated sentence with a count still gets the count."""
    bad = 0
    for lang in ("en", "uk"):
        text = mod.CAT.tr(lang, "ui.attention", n=7)
        bad += check(f"{lang} fills the count", "7" in text and "{" not in text, text)
    return bad


def test_manifest_says_windows():
    """The gate that stops this module answering 'nothing is licensed' on Linux."""
    toml = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "limen.toml"),
                encoding="utf-8").read()
    bad = check("the manifest declares its os", 'os = ["windows"]' in toml)
    bad += check("and asks for no more than it needs",
                 "network = true" not in toml and "elevate = true" not in toml)
    return bad


if __name__ == "__main__":
    failed = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            print(name)
            failed += fn() or 0
    print("\n" + ("all good" if not failed else f"{failed} failure(s)"))
    sys.exit(1 if failed else 0)
