use crate::discovery::types::AppEntry;

use super::CommandRegistry;

pub enum DispatchResult {
    CommandExecuted,
    AppSearch(Vec<AppEntry>),
}

pub fn dispatch(query: &str, registry: &CommandRegistry) -> DispatchResult {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return DispatchResult::AppSearch(Vec::new());
    }

    let (cmd_name, args) = match trimmed.split_once(' ') {
        Some((name, args)) => (name, args),
        None => (trimmed, ""),
    };

    if let Some(cmd) = registry.get(cmd_name) {
        cmd.execute(args);
        DispatchResult::CommandExecuted
    } else {
        DispatchResult::AppSearch(Vec::new())
    }
}
