// Plugin engine lives in the openusage-plugin-engine workspace crate so the
// headless agent can reuse it. Re-export everything so existing references
// inside src-tauri (e.g. plugin_engine::runtime::PluginOutput) keep working.
pub use openusage_plugin_engine::*;
