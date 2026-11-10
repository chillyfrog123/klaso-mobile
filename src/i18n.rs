use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU8, Ordering},
};

/// Process-wide UI preference, also available to API worker threads.
static ACTIVE: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Polish,
}
impl Language {
    pub fn current() -> Self {
        #[cfg(test)]
        if let Some(language) = TEST_LANGUAGE.with(|value| value.get()) {
            return language;
        }
        if ACTIVE.load(Ordering::Relaxed) == 1 {
            Self::Polish
        } else {
            Self::English
        }
    }
    pub fn activate(self) {
        ACTIVE.store(u8::from(self == Self::Polish), Ordering::Relaxed);
    }
    fn path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(|root| PathBuf::from(root).join("LibrusLite").join("language.json"))
    }
    fn read(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn load() -> Self {
        Self::path()
            .map(|path| Self::read(&path))
            .unwrap_or_default()
    }
    fn write(self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec(&self)?)
    }
    pub fn save(self) -> std::io::Result<()> {
        self.write(
            &Self::path().ok_or_else(|| {
                std::io::Error::other("Application data directory is unavailable")
            })?,
        )
    }
}

// Both alternatives live at the call site so format! checks their arguments at
// compile time. No translated text is ever used as a protocol key or field ID.
macro_rules! tr {
    ($pl:literal, $en:literal) => {
        if $crate::i18n::Language::current() == $crate::i18n::Language::Polish {
            $pl
        } else {
            $en
        }
    };
}
macro_rules! trf {
    ($pl:literal, $en:literal $(, $($args:tt)*)?) => {
        if $crate::i18n::Language::current() == $crate::i18n::Language::Polish {
            format!($pl $(, $($args)*)?)
        } else {
            format!($en $(, $($args)*)?)
        }
    };
}
pub(crate) use {tr, trf};

#[cfg(test)]
thread_local! { static TEST_LANGUAGE: std::cell::Cell<Option<Language>> = const { std::cell::Cell::new(None) }; }
#[cfg(test)]
pub fn with_language<T>(language: Language, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<Language>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_LANGUAGE.with(|value| value.set(self.0));
        }
    }
    let _restore = Restore(TEST_LANGUAGE.with(|value| value.replace(Some(language))));
    run()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_is_default_and_preferences_round_trip_without_touching_real_settings() {
        assert_eq!(Language::default(), Language::English);
        let path = std::env::temp_dir().join(format!(
            "librus-language-{}-{}.json",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        assert_eq!(Language::read(&path), Language::English);
        Language::Polish.write(&path).unwrap();
        assert_eq!(Language::read(&path), Language::Polish);
        Language::English.write(&path).unwrap();
        assert_eq!(Language::read(&path), Language::English);
        std::fs::write(&path, "invalid").unwrap();
        assert_eq!(Language::read(&path), Language::English);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn translations_switch_immediately_and_keep_dynamic_values() {
        let name = "Alice";
        for language in [Language::English, Language::Polish] {
            with_language(language, || {
                assert_eq!(
                    tr!("Oceny", "Grades"),
                    if language == Language::English {
                        "Grades"
                    } else {
                        "Oceny"
                    }
                );
                assert_eq!(
                    trf!("Witaj {name}: {}", "Hello {name}: {}", 7),
                    if language == Language::English {
                        "Hello Alice: 7"
                    } else {
                        "Witaj Alice: 7"
                    }
                );
            });
        }
    }
}
