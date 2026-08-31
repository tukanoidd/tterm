use url::Url;

/// Minimal HTML escaping for error pages.
pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Check if two URLs refer to the same page (ignoring fragment).
pub(crate) fn is_same_page(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host() == b.host()
        && a.port() == b.port()
        && a.path() == b.path()
        && a.query() == b.query()
}
