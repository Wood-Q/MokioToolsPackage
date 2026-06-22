//! Tiny GitHub REST client over curl. We deliberately shell out to `curl`
//! (ubiquitous on macOS) instead of pulling in a heavy HTTP stack.

use serde_json::Value;

use crate::shell;

fn api_get(path: &str) -> Option<String> {
    let url = format!("https://api.github.com/{path}");
    shell::capture(
        "curl",
        &[
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: mokio-bootstrap",
            &url,
        ],
    )
}

/// The latest release JSON for `owner/repo`, or `None` on any failure
/// (network error, rate limit, etc.).
pub fn latest_release(owner: &str, repo: &str) -> Option<Value> {
    let body = api_get(&format!("repos/{owner}/{repo}/releases/latest"))?;
    serde_json::from_str(&body).ok()
}

/// `tag_name` of the latest release.
pub fn latest_tag(owner: &str, repo: &str) -> Option<String> {
    latest_release(owner, repo)?
        .get("tag_name")?
        .as_str()
        .map(str::to_string)
}

/// First `browser_download_url` whose name passes `predicate`.
pub fn asset_url<F>(release: &Value, predicate: F) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let assets = release.get("assets")?.as_array()?;
    for a in assets {
        let name = a.get("name")?.as_str()?;
        if predicate(name) {
            return a.get("browser_download_url")?.as_str().map(str::to_string);
        }
    }
    None
}
