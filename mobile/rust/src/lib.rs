//! Shared protocol implementation: source-linked, never a second HTTP client.
#[path = "../../../src/api.rs"]
pub mod api;
#[path = "../../../src/data.rs"]
pub mod data;
#[path = "../../../src/demo.rs"]
pub mod demo;
#[path = "../../../src/i18n.rs"]
pub mod i18n;
pub mod vault {
    use serde::{Deserialize, Serialize};
    use zeroize::Zeroize;
    #[derive(Serialize, Deserialize)]
    pub struct Credentials {
        pub login: String,
        pub password: String,
    }
    impl Drop for Credentials {
        fn drop(&mut self) {
            self.password.zeroize();
        }
    }
    #[cfg(test)]
    pub fn load() -> anyhow::Result<Option<Credentials>> {
        Ok(None)
    }
}
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
static SESSION: OnceLock<Mutex<Option<api::Session>>> = OnceLock::new();

// Complete the public library collection API without changing the desktop model.
impl data::Payload {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn enrich(payload: data::Payload) -> Value {
    let mut output = serde_json::to_value(&payload).unwrap();
    if let data::Payload::Grades { items, .. } = &payload {
        let mut subjects: Vec<_> = items.iter().map(|g| g.subject.clone()).collect();
        subjects.sort();
        subjects.dedup();
        let summaries: Vec<_> = (0..=2)
            .map(|semester| {
                let pair = |rows: Vec<&data::Grade>| {
                    json!([
                        data::grade_average(rows.clone(), semester, false),
                        data::grade_average(rows, semester, true)
                    ])
                };
                json!({"overall": pair(items.iter().collect()), "subjects": subjects.iter().map(|s|
                (s.clone(), pair(items.iter().filter(|g| &g.subject == s).collect()))
            ).collect::<serde_json::Map<String,Value>>()})
            })
            .collect();
        output["Grades"]["summaries"] = json!(summaries);
    }
    output
}
/// JSON boundary is portable to Swift/C; Android adds only the JNI string adapter.
pub fn dispatch(request: Value) -> Result<Value> {
    let s = |key: &str| request[key].as_str().unwrap_or("");
    let week = || chrono::NaiveDate::parse_from_str(s("week"), "%Y-%m-%d").context("Invalid week");
    match s("op") {
        "language" => {
            if s("language") == "pl" {
                i18n::Language::Polish
            } else {
                i18n::Language::English
            }
            .activate();
            return Ok(json!({}));
        }
        "demo" => {
            return Ok(json!(demo::payloads(week()?)
                .into_iter()
                .map(|(section, p)| (format!("{section:?}"), enrich(p)))
                .collect::<serde_json::Map<String, Value>>()))
        }
        _ => (),
    }
    let mut session = SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| anyhow::anyhow!("Session unavailable"))?;
    match s("op") {
        "login" => {
            *session = None;
            *session = Some(api::Session::login(s("login"), s("password"))?);
            Ok(json!({"authenticated":true}))
        }
        "restore" => {
            *session = Some(api::Session::deferred(s("login"), s("password"))?);
            Ok(json!({"authenticated":false}))
        }
        "logout" => {
            if let Some(old) = session.take() {
                let _ = old.logout();
            }
            Ok(json!({}))
        }
        "drop" => {
            *session = None;
            Ok(json!({}))
        }
        "load" => {
            let section: data::Section = serde_json::from_value(request["section"].clone())?;
            let page = request["page"].as_u64().unwrap_or(1).max(1) as usize;
            let payload =
                session
                    .as_ref()
                    .context("Sign in required")?
                    .load(section, week()?, page)?;
            // Treat partial behaviour failure as failed refresh: retain the complete previous snapshot.
            if let data::Payload::Grades {
                behaviour_error: Some(_),
                ..
            } = &payload
            {
                bail!("Incomplete grade refresh");
            }
            Ok(enrich(payload))
        }
        "detail" => Ok(serde_json::to_value(
            session
                .as_ref()
                .context("Sign in required")?
                .message_detail(s("id"))?,
        )?),
        _ => bail!("Unknown operation"),
    }
}
pub fn respond(input: &str) -> String {
    let result = serde_json::from_str(input)
        .map_err(anyhow::Error::from)
        .and_then(dispatch);
    match result {
        Ok(value) => json!({"ok":true,"value":value}).to_string(),
        Err(error) => {
            // Do not return URLs, server bodies or transport diagnostics through the UI bridge.
            let code = error_code(&error);
            json!({"ok":false,"code":code}).to_string()
        }
    }
}
fn error_code(error: &anyhow::Error) -> &'static str {
    for cause in error.chain() {
        let message = cause.to_string().to_lowercase();
        if [
            "captcha",
            "verification",
            "password change",
            "known leaks",
            "account setup",
            "browser challenge",
        ]
        .iter()
        .any(|marker| message.contains(marker))
        {
            return "verification";
        }
        if message.contains("sign in required") {
            return "signedOut";
        }
    }
    "requestFailed"
}
/// Portable C boundary for an iOS adapter. Calls must run off the main thread.
/// # Safety
/// `input` must point to a valid, NUL-terminated UTF-8 string for this call.
/// Release the returned pointer with `librus_mobile_free` exactly once.
#[no_mangle]
pub unsafe extern "C" fn librus_mobile_request(
    input: *const std::ffi::c_char,
) -> *mut std::ffi::c_char {
    let result = std::panic::catch_unwind(|| {
        if input.is_null() {
            return "{\"ok\":false,\"code\":\"requestFailed\"}".to_string();
        }
        let request = unsafe { std::ffi::CStr::from_ptr(input) }
            .to_str()
            .unwrap_or("");
        respond(request)
    })
    .unwrap_or_else(|_| "{\"ok\":false,\"code\":\"requestFailed\"}".into());
    std::ffi::CString::new(result)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
/// # Safety
/// `pointer` must be null or an unreleased pointer returned by `librus_mobile_request`.
#[no_mangle]
pub unsafe extern "C" fn librus_mobile_free(pointer: *mut std::ffi::c_char) {
    use zeroize::Zeroize;
    if !pointer.is_null() {
        let mut bytes = unsafe { std::ffi::CString::from_raw(pointer) }.into_bytes_with_nul();
        bytes.zeroize();
    }
}
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_pl_libruslite_mobile_LibrusPlugin_nativeRequest(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    input: jni::objects::JString,
) -> jni::sys::jstring {
    use zeroize::Zeroize;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut raw: String = env.get_string(&input).map(|s| s.into()).unwrap_or_default();
        let response = respond(&raw);
        raw.zeroize();
        response
    }))
    .unwrap_or_else(|_| "{\"ok\":false,\"code\":\"requestFailed\"}".into());
    env.new_string(result)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]
mod bridge_tests {
    use super::*;
    #[test]
    fn demo_uses_shared_averages_and_no_session() {
        let data = dispatch(json!({"op":"demo","week":"2026-11-09"})).unwrap();
        assert!(
            data["Grades"]["Grades"]["summaries"][0]["overall"][0]
                .as_f64()
                .unwrap()
                > 4.
        );
    }
    #[test]
    fn errors_never_expose_input() {
        assert!(
            !respond(r#"{"op":"invalid","password":"private-secret"}"#).contains("private-secret")
        );
    }
    #[test]
    fn required_actions_survive_session_renewal_context() {
        let error = anyhow::anyhow!("Librus requires CAPTCHA").context("Could not renew session");
        assert_eq!(error_code(&error), "verification");
        let error = anyhow::anyhow!("A browser challenge may be required");
        assert_eq!(error_code(&error), "verification");
    }
}
