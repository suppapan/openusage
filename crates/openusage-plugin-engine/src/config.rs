// Pluggable proxy configuration: callers (src-tauri or the agent) install a
// resolver via `set_proxy_resolver`. The plugin runtime calls
// `get_resolved_proxy` from inside the HTTP host API.
//
// We keep this here (rather than a generic trait/extension point) because the
// runtime is shared and needs a stable, no-arg lookup at request time.

use std::sync::OnceLock;

#[derive(Clone)]
pub struct ResolvedProxy {
    pub proxy: reqwest::Proxy,
}

type ProxyResolver = fn() -> Option<&'static ResolvedProxy>;

static RESOLVER: OnceLock<ProxyResolver> = OnceLock::new();

/// Install a function that returns the active proxy (if any). Should be called
/// once at startup. If not called, `get_resolved_proxy` returns None and HTTP
/// calls go direct.
pub fn set_proxy_resolver(resolver: ProxyResolver) {
    let _ = RESOLVER.set(resolver);
}

pub fn get_resolved_proxy() -> Option<&'static ResolvedProxy> {
    let resolver = RESOLVER.get()?;
    resolver()
}
