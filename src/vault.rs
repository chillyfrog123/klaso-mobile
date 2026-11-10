use anyhow::Result;
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
fn entry() -> Result<keyring::Entry> {
    Ok(keyring::Entry::new("LibrusLite", "remembered-account")?)
}
pub fn load() -> Result<Option<Credentials>> {
    match entry()?.get_password() {
        Ok(mut text) => {
            let result = serde_json::from_str(&text);
            text.zeroize();
            Ok(Some(result?))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn save(credentials: &Credentials) -> Result<()> {
    let mut text = serde_json::to_string(credentials)?;
    let result = entry()?.set_password(&text);
    text.zeroize();
    Ok(result?)
}
pub fn forget() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
