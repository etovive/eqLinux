//! Minimal English/Spanish translations for the interface.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Spanish,
}

impl Language {
    pub fn detect() -> Self {
        let value = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        if value.starts_with("es") {
            Self::Spanish
        } else {
            Self::English
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
        }
    }

    pub fn t(self, english: &'static str, spanish: &'static str) -> &'static str {
        match self {
            Self::English => english,
            Self::Spanish => spanish,
        }
    }
}
