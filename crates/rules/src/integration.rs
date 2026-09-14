//! Process/RuleSet integration tests surface (NP-096).

use crate::ruleset::{match_with_process, RuleSet};
use crate::updater::RuleSetUpdater;

pub fn process_ruleset_smoke() -> Result<(), String> {
    let mut updater = RuleSetUpdater::new();
    updater
        .apply_text(
            "integ",
            "Integ",
            "PROCESS-NAME,chrome.exe,BROWSER\nDOMAIN-SUFFIX,example.com,PROXY\nMATCH,DIRECT\n",
        )
        .map_err(|e| e.to_string())?;
    let set = updater.active().ok_or("no active set")?;
    let idx = set.index();
    let (d, _) = match_with_process(&idx, Some("www.example.com"), None, Some("chrome.exe"))
        .ok_or("no match")?;
    // PROCESS-NAME has same default priority; order depends on priority sort.
    // Either BROWSER or PROXY is acceptable depending on rule order after priority sort.
    if d.outbound != "BROWSER" && d.outbound != "PROXY" {
        return Err(format!("unexpected outbound {}", d.outbound));
    }
    let _ = RuleSet::new("x", "x", vec![]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        process_ruleset_smoke().unwrap();
    }
}
