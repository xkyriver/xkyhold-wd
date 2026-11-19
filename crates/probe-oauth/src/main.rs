//! Phase 0 probe (risk c), throwaway. Answers, on Windows:
//!
//! 1. Does the "XkyHold-wd" Desktop client sign in with PKCE and a loopback
//!    redirect, with only the `drive.file` scope, from the system browser?
//! 2. Does it get a refresh token, store it in the Windows Credential
//!    Manager, read it back, and use it?
//! 3. With `drive.file`, does it see the folders XkyHold web and XkyHold-lx
//!    created (same Google Cloud project, different clients)?
//!
//! Client id and secret come from the environment (built in by the real
//! app), never from git. It prints no token and never writes to Drive.
//! `--forget` deletes the stored test token.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use keyring_core::Entry;
use serde_json::Value;
use sha2::{Digest, Sha256};

const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
const API: &str = "https://www.googleapis.com/drive/v3";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const FOLDER: &str = "application/vnd.google-apps.folder";
const SERVICE: &str = "xkyhold-wd";
const KEY: &str = "google-refresh:probe";

fn random(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    getrandom::fill(&mut bytes).expect("system randomness");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| {
            let bytes = v.replace('+', " ").into_bytes();
            let mut out = Vec::new();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    out.push(
                        u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap(), 16)
                            .unwrap(),
                    );
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            String::from_utf8(out).unwrap()
        })
    })
}

fn token(form: &[(&str, &str)]) -> Value {
    let body = form
        .iter()
        .map(|(k, v)| format!("{k}={}", enc(v)))
        .collect::<Vec<_>>()
        .join("&");
    let mut res = ureq::post(TOKEN)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .config()
        .http_status_as_error(false)
        .build()
        .send(body)
        .expect("token request");
    let status = res.status();
    let json: Value = res.body_mut().read_json().expect("token JSON");
    if !status.is_success() {
        panic!("token endpoint said {status}: {}", json["error"]);
    }
    json
}

fn get(access: &str, url: &str) -> Value {
    ureq::get(url)
        .header("Authorization", &format!("Bearer {access}"))
        .call()
        .expect("Drive request")
        .body_mut()
        .read_json()
        .expect("Drive JSON")
}

fn credential() -> Entry {
    #[cfg(windows)]
    keyring_core::set_default_store(
        windows_native_keyring_store::Store::new().expect("Credential Manager"),
    );
    Entry::new(SERVICE, KEY).expect("credential entry")
}

fn open_browser(url: &str) {
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .status();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(url).status();
}

fn main() {
    let entry = credential();
    if std::env::args().any(|a| a == "--forget") {
        let _ = entry.delete_credential();
        println!("Stored test token deleted.");
        return;
    }
    let client_id = std::env::var("XKYHOLD_WD_CLIENT_ID").expect("XKYHOLD_WD_CLIENT_ID");
    let client_secret =
        std::env::var("XKYHOLD_WD_CLIENT_SECRET").expect("XKYHOLD_WD_CLIENT_SECRET");

    // 1. PKCE + loopback redirect on a free port, system browser.
    let verifier = random(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random(16);
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback port");
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let url = format!(
        "{AUTH}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={challenge}\
         &code_challenge_method=S256&state={state}&access_type=offline&prompt=consent",
        enc(&client_id),
        enc(&redirect),
        enc(SCOPE),
    );
    println!("Opening the sign-in page in the default browser.");
    open_browser(&url);

    let (mut stream, _) = listener.accept().expect("redirect");
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).unwrap();
    let query = line
        .split_whitespace()
        .nth(1)
        .and_then(|p| p.split_once('?'))
        .map(|(_, q)| q)
        .unwrap_or("");
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
         <p>XkyHold-wd probe: you can close this tab.</p>"
    );
    assert_eq!(
        query_param(query, "state").as_deref(),
        Some(state.as_str()),
        "state mismatch"
    );
    if let Some(err) = query_param(query, "error") {
        panic!("Google said: {err}");
    }
    let code = query_param(query, "code").expect("authorization code");

    // 2. Code exchange; the refresh token goes to the Credential Manager and
    //    is read back from there for the refresh.
    let first = token(&[
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
        ("code_verifier", &verifier),
        ("redirect_uri", &redirect),
    ]);
    println!("Scopes granted: {}", first["scope"]);
    let refresh = first["refresh_token"]
        .as_str()
        .expect("no refresh token in the response");
    entry
        .set_secret(refresh.as_bytes())
        .expect("store refresh token");
    println!("Refresh token: received and stored in the Credential Manager (not shown).");
    let stored = String::from_utf8(entry.get_secret().expect("read refresh token back")).unwrap();
    assert_eq!(stored, refresh, "stored token differs");
    println!("Refresh token: read back from the Credential Manager, identical.");
    let second = token(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", &stored),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
    ]);
    let access = second["access_token"]
        .as_str()
        .expect("access token after refresh");
    println!(
        "Refresh: new access token received, valid for {} s.",
        second["expires_in"]
    );

    // 3. What does drive.file let this client see? Read only.
    let about = get(access, &format!("{API}/about?fields=user(emailAddress)"));
    let email = about["user"]["emailAddress"].as_str().unwrap_or("?");
    let masked = email
        .split_once('@')
        .map(|(u, d)| format!("{}…@{d}", &u[..u.len().min(2)]))
        .unwrap_or_default();
    println!("Signed in as: {masked}");
    let q = format!(
        "name = 'XkyHold' and mimeType = '{FOLDER}' and 'root' in parents and trashed = false"
    );
    let found = get(
        access,
        &format!("{API}/files?q={}&fields=files(id)", enc(&q)),
    );
    let folders = found["files"].as_array().cloned().unwrap_or_default();
    println!(
        "Visible 'XkyHold' folders in the Drive root: {}",
        folders.len()
    );
    for folder in &folders {
        let id = folder["id"].as_str().unwrap();
        let q = format!("'{id}' in parents and trashed = false");
        let children = get(
            access,
            &format!("{API}/files?q={}&fields=files(name,createdTime)", enc(&q)),
        );
        for child in children["files"].as_array().unwrap() {
            println!(
                "  XkyHold/{}  (created {})",
                child["name"].as_str().unwrap(),
                child["createdTime"]
            );
        }
    }
}
