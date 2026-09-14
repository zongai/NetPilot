# Process & RuleSet (S7, NP-085…NP-090)

## Process (`netpilot-process`)

| NP | API |
|----|-----|
| 085 | `ProcessResolver` / `MockProcessTable` |
| 086 | `ProcessIdentity` |
| 087 | `normalize_exe_path` / `path_matches` |
| 088 | `PidTracker` / `PidEvent` |

Windows enumeration APIs are deferred; CI uses the mock table.

## RuleSet (`netpilot-rules`)

| NP | API |
|----|-----|
| 089 | `match_with_process` |
| 090 | `RuleSet` named collection + `from_text` |
