//! The registry of all installers and selection/ordering helpers.

use crate::installer::{Installer, ToolInfo};
use crate::tools::*;

/// The full set of installers, ordered by `ToolInfo::order`.
pub struct Catalog {
    installers: Vec<Box<dyn Installer>>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog {
    pub fn new() -> Self {
        let installers: Vec<Box<dyn Installer>> = vec![
            Box::new(Homebrew),
            Box::new(Git),
            Box::new(Node),
            Box::new(Uv),
            Box::new(VsCode),
            Box::new(Chrome),
            Box::new(Terminal),
            Box::new(Codex),
            Box::new(ClaudeCode),
            Box::new(CcSwitch),
        ];
        Self { installers }
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Installer> {
        self.installers.iter().map(|b| b.as_ref())
    }

    pub fn infos(&self) -> Vec<ToolInfo> {
        let mut v: Vec<ToolInfo> = self.installers.iter().map(|i| i.info()).collect();
        v.sort_by_key(|i| i.order);
        v
    }

    pub fn get(&self, id: &str) -> Option<&dyn Installer> {
        self.installers
            .iter()
            .map(|b| b.as_ref())
            .find(|i| i.info().id == id)
    }

    /// `ids` plus any prerequisites (transitively). Result is ordered by
    /// `ToolInfo::order` so dependencies run first.
    pub fn expand_with_deps(&self, ids: &[String]) -> Vec<String> {
        let infos: Vec<ToolInfo> = self.infos();
        let mut set: std::collections::BTreeSet<String> = ids.iter().cloned().collect();
        let mut changed = true;
        while changed {
            changed = false;
            for info in &infos {
                if set.contains(&info.id) {
                    for r in &info.requires {
                        if set.insert(r.clone()) {
                            changed = true;
                        }
                    }
                }
            }
        }
        let mut out: Vec<String> = set.into_iter().collect();
        out.sort_by_key(|id| {
            infos
                .iter()
                .find(|i| &i.id == id)
                .map(|i| i.order)
                .unwrap_or(u32::MAX)
        });
        out
    }
}
