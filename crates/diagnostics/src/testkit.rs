//! Inspector and script testkit (NP-108).

use crate::connection::ConnectionManager;
use crate::inspector::InspectorApi;
use crate::script::{ScriptModule, ScriptSandbox};

pub fn inspector_script_smoke() -> Result<(), String> {
    let mut mgr = ConnectionManager::new();
    mgr.open(
        "example.com:443",
        "tcp",
        Some("app.exe".into()),
        "PROXY",
        None,
    );
    let snap = InspectorApi::snapshot(&mgr);
    if snap.connection_count != 1 {
        return Err("expected 1 connection".into());
    }
    let sb = ScriptSandbox::default();
    sb.execute(&ScriptModule {
        name: "noop".into(),
        source: "return".into(),
    })
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        inspector_script_smoke().unwrap();
    }
}
