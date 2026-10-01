//! Signing in to Epic.
//!
//! Epic has no public OAuth client for third parties, so every open source
//! client reuses the flow the official launcher uses. Legendary registers a
//! client id and offers two ways in: an embedded browser that captures the
//! code, or pasting the code by hand.
//!
//! Wisp does the same, with its own window instead of Legendary's:
//!
//! 1. Open Epic's real login page in a separate webview. Nothing is proxied
//!    and Wisp never sees the password.
//! 2. After signing in, Epic redirects to its own `/id/api/redirect` endpoint,
//!    which answers with a small JSON document containing a one-time
//!    authorization code.
//! 3. Wisp reads that code out of the page and hands it to
//!    `legendary auth --code`, which exchanges it for the tokens it stores.
//!
//! Signing in through Xbox, Google, Steam and the rest goes through a popup
//! that Epic opens with `window.open`, so those are given real windows of
//! their own and watched the same way.
//!
//! If anything in that chain breaks, the same page can be opened in the
//! system browser and the code pasted into Wisp by hand, which is exactly
//! what `legendary auth --disable-webview` asks for.

use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Serialize;
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{Error, Result};
use crate::legendary::Legendary;

/// Emitted whenever the signed-in account changes.
pub const EVENT: &str = "auth:changed";

/// Label of the login webview, so it is never opened twice.
pub const LOGIN_WINDOW: &str = "epic-login";

/// Legendary's registered client id. Epic only redirects back for clients it
/// knows, so this cannot be an arbitrary value.
const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";

/// Path Epic lands on once the user has signed in.
const REDIRECT_PATH: &str = "/id/api/redirect";

/// Reads the signed-in state out of the page Epic redirects to.
const READ_CODE_JS: &str = "document.body ? document.body.innerText : ''";

/// Gives every provider popup a label of its own.
static POPUP_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    pub logged_in: bool,
    pub account: Option<String>,
    pub games_available: u32,
}

/// URL that shows Epic's login form and comes back with a code.
pub fn login_url() -> String {
    let redirect = urlencoding(&format!(
        "https://www.epicgames.com/id/api/redirect?clientId={CLIENT_ID}&responseType=code"
    ));
    format!("https://www.epicgames.com/id/login?redirectUrl={redirect}")
}

/// Epic keeps a session cookie, so going straight to the login page would
/// silently reuse the last account. Starting at logout makes the account
/// chooser appear, which is also how Legendary does it on Windows.
fn start_url() -> String {
    format!(
        "https://www.epicgames.com/id/logout?productName=epic-games&redirectUrl={}",
        urlencoding(&login_url())
    )
}

/// Current account, according to Legendary.
pub async fn status(legendary: &Legendary) -> Result<AuthState> {
    let status = legendary.status().await?;
    Ok(AuthState {
        logged_in: status.logged_in(),
        account: status.account_name().map(str::to_owned),
        games_available: status.games_available,
    })
}

/// Opens the Epic login window, or focuses it if it is already open.
pub fn open_login_window(app: &AppHandle) -> Result<()> {
    if let Some(existing) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = existing.set_focus();
        return Ok(());
    }

    let url = start_url()
        .parse()
        .map_err(|e| Error::Config(format!("the Epic login URL is not valid: {e}")))?;

    tracing::info!("opening the Epic login window");
    let handle = app.clone();
    let popup_handle = app.clone();

    WebviewWindowBuilder::new(app, LOGIN_WINDOW, WebviewUrl::External(url))
        .title("Epic Games")
        // Epic's login form is built for a narrow, tall viewport.
        .inner_size(520.0, 780.0)
        .min_inner_size(420.0, 560.0)
        .center()
        .on_page_load(move |window, payload| watch_for_code(&handle, &window, &payload))
        // "Sign in with Xbox", Google, Steam and the rest hand off to the
        // provider in a popup. Without a handler for `window.open` those
        // buttons silently do nothing, so each one gets a real window.
        .on_new_window(move |url, features| open_provider_popup(&popup_handle, &url, features))
        .build()
        .map_err(|e| Error::Config(format!("could not open the login window: {e}")))?;

    Ok(())
}

/// Opens one of the sign-in providers Epic delegates to.
///
/// The popup keeps its link to the page that opened it, which the provider
/// flows rely on to report back, and it inherits the same page watcher so a
/// flow that finishes inside the popup is still picked up.
fn open_provider_popup(
    app: &AppHandle,
    url: &Url,
    features: tauri::webview::NewWindowFeatures,
) -> NewWindowResponse<tauri::Wry> {
    let label = format!(
        "{LOGIN_WINDOW}-popup-{}",
        POPUP_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    tracing::info!(provider = %url.host_str().unwrap_or("unknown"), "opening a sign-in popup");

    let watcher = app.clone();
    let builder = WebviewWindowBuilder::new(
        app,
        label,
        // The popup starts blank and the webview then navigates it to the
        // URL the page asked for, which is what keeps the two linked.
        WebviewUrl::External("about:blank".parse().expect("about:blank is a valid URL")),
    )
    .title(url.host_str().unwrap_or("Sign in"))
    .window_features(features)
    .on_document_title_changed(|window, title| {
        let _ = window.set_title(&title);
    })
    .on_page_load(move |window, payload| watch_for_code(&watcher, &window, &payload));

    match builder.build() {
        Ok(window) => NewWindowResponse::Create { window },
        Err(e) => {
            tracing::error!("could not open the sign-in popup: {e}");
            NewWindowResponse::Deny
        }
    }
}

/// Picks the authorization code out of Epic's redirect page.
///
/// Attached to every window in the login flow, because a provider popup can
/// be the one that ends up on the redirect.
fn watch_for_code(
    app: &AppHandle,
    window: &WebviewWindow,
    payload: &tauri::webview::PageLoadPayload<'_>,
) {
    tracing::debug!(
        url = %payload.url().as_str().split('?').next().unwrap_or_default(),
        event = ?payload.event(),
        "login page load"
    );

    if payload.url().path() != REDIRECT_PATH {
        return;
    }

    let app = app.clone();
    // The code lives in the page body, so it has to be read out of the
    // webview rather than parsed from the URL.
    let _ = window.eval_with_callback(READ_CODE_JS, move |result| {
        let Some(code) = authorization_code(&result) else {
            return;
        };
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            finish_login(app, code).await;
        });
    });
}

/// Closes the login window and any provider popup left open with it.
fn close_login_windows(app: &AppHandle) {
    let open: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(LOGIN_WINDOW))
        .map(|(_, window)| window)
        .collect();

    for window in open {
        let _ = window.close();
    }
}

/// Hands a code to Legendary and tells the app how it went.
async fn finish_login(app: AppHandle, code: String) {
    let state = app.state::<crate::AppState>();
    let legendary = state.runtime.legendary();

    let result = match legendary.auth_with_code(&code).await {
        Ok(()) => status(&legendary).await,
        Err(e) => Err(e),
    };

    close_login_windows(&app);

    match result {
        Ok(state) => {
            tracing::info!(logged_in = state.logged_in, "login finished");
            let _ = app.emit(EVENT, &state);
        }
        Err(e) => {
            tracing::error!("login failed: {e}");
            let _ = app.emit(
                EVENT,
                &LoginFailed {
                    error: e.to_string(),
                },
            );
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginFailed {
    error: String,
}

/// Completes a login from a code the user pasted in by hand.
pub async fn submit_code(legendary: &Legendary, code: &str) -> Result<AuthState> {
    let code = code.trim();
    if code.is_empty() {
        return Err(Error::Config("no authorization code was given".into()));
    }
    legendary.auth_with_code(code).await?;
    status(legendary).await
}

pub async fn logout(legendary: &Legendary) -> Result<AuthState> {
    legendary.logout().await?;
    status(legendary).await
}

/// Digs the authorization code out of whatever the webview returned.
///
/// `eval_with_callback` hands back the JSON-encoded value of the expression,
/// so the usual shape is a JSON string containing the page text, which is
/// itself JSON. Both levels are handled, and so is the case where Epic
/// answers with no code because the user is not signed in yet.
pub fn authorization_code(evaluated: &str) -> Option<String> {
    let unquoted =
        serde_json::from_str::<String>(evaluated).unwrap_or_else(|_| evaluated.to_owned());

    let document: serde_json::Value = serde_json::from_str(unquoted.trim())
        .or_else(|_| serde_json::from_str(evaluated.trim()))
        .ok()?;

    document
        .get("authorizationCode")?
        .as_str()
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
}

/// Percent-encodes a URL for use as a query parameter.
/// Only the characters that appear in these URLs need escaping.
fn urlencoding(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_code_out_of_a_json_string_result() {
        // What the webview actually returns: a JSON string whose contents
        // are the JSON document Epic served.
        let evaluated =
            r#""{\"redirectUrl\":\"https://x\",\"authorizationCode\":\"abc123\",\"sid\":null}""#;
        assert_eq!(authorization_code(evaluated).as_deref(), Some("abc123"));
    }

    #[test]
    fn reads_the_code_out_of_a_bare_json_object() {
        let evaluated = r#"{"authorizationCode":"abc123"}"#;
        assert_eq!(authorization_code(evaluated).as_deref(), Some("abc123"));
    }

    #[test]
    fn no_code_yet_is_not_a_code() {
        assert_eq!(
            authorization_code(r#""{\"authorizationCode\":null}""#),
            None
        );
        assert_eq!(
            authorization_code(r#""{\"authorizationCode\":\"\"}""#),
            None
        );
        assert_eq!(authorization_code(r#""{\"sid\":\"x\"}""#), None);
    }

    #[test]
    fn page_text_that_is_not_json_is_ignored() {
        assert_eq!(authorization_code(r#""<html>sign in</html>""#), None);
        assert_eq!(authorization_code(""), None);
        assert_eq!(authorization_code("null"), None);
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let evaluated = "\"  {\\\"authorizationCode\\\":\\\" abc \\\"}  \"";
        assert_eq!(authorization_code(evaluated).as_deref(), Some("abc"));
    }

    #[test]
    fn the_login_url_asks_epic_for_a_code_for_legendarys_client() {
        let url = login_url();
        assert!(url.starts_with("https://www.epicgames.com/id/login?redirectUrl="));
        assert!(url.contains(CLIENT_ID));
        assert!(url.contains("responseType%3Dcode"));
    }

    #[test]
    fn the_start_url_logs_out_first_so_accounts_can_be_switched() {
        let url = start_url();
        assert!(url.starts_with("https://www.epicgames.com/id/logout?"));
        assert!(url.contains("redirectUrl=https%3A%2F%2Fwww.epicgames.com%2Fid%2Flogin"));
    }

    #[test]
    fn urlencoding_escapes_everything_a_url_needs() {
        assert_eq!(urlencoding("a/b?c=d&e"), "a%2Fb%3Fc%3Dd%26e");
        assert_eq!(urlencoding("safe-._~"), "safe-._~");
    }
}
