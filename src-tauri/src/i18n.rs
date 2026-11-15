//! The UI language (ADR 0050): English or German, chosen in Settings → Appearance. Every text the window
//! shows is written in both languages where it is built, so a missing translation fails to compile. The CLI
//! and the logs stay English.

use std::cell::Cell;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::settings::Language;

static CURRENT: AtomicU8 = AtomicU8::new(0);

thread_local! {
    static OVERRIDE: Cell<Option<Language>> = const { Cell::new(None) };
}

fn to_u8(language: Language) -> u8 {
    match language {
        Language::En => 0,
        Language::De => 1,
    }
}

/// The language texts are built in: this thread's `with_lang` if any, else the app's setting.
pub fn lang() -> Language {
    OVERRIDE
        .with(Cell::get)
        .unwrap_or_else(|| match CURRENT.load(Ordering::Relaxed) {
            1 => Language::De,
            _ => Language::En,
        })
}

/// Follows the setting; called at launch and when the owner switches.
pub fn set(language: Language) {
    CURRENT.store(to_u8(language), Ordering::Relaxed);
}

/// Runs `f` with texts in `language` on this thread only, so tests in parallel never see each other's choice.
pub fn with_lang<T>(language: Language, f: impl FnOnce() -> T) -> T {
    let before = OVERRIDE.with(|o| o.replace(Some(language)));
    let result = f();
    OVERRIDE.with(|o| o.set(before));
    result
}

/// A fixed text in both languages, for tables built at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    pub en: &'static str,
    pub de: &'static str,
}

impl Text {
    pub const fn new(en: &'static str, de: &'static str) -> Self {
        Self { en, de }
    }

    pub fn get(&self) -> &'static str {
        match lang() {
            Language::En => self.en,
            Language::De => self.de,
        }
    }
}

/// `tr!("Undo", "Widerrufen")`, `tr!("{n} lines", "{n} Buchungen")`: the text in the current language, formatted
/// like `format!` (inline names and trailing arguments both work).
#[macro_export]
macro_rules! tr {
    ($en:literal, $de:literal $(,)?) => {
        match $crate::i18n::lang() {
            $crate::settings::Language::En => format!($en),
            $crate::settings::Language::De => format!($de),
        }
    };
    ($en:literal, $de:literal, $($arg:tt)+) => {
        match $crate::i18n::lang() {
            $crate::settings::Language::En => format!($en, $($arg)+),
            $crate::settings::Language::De => format!($de, $($arg)+),
        }
    };
}

/// `tr_plural!(n, "{n} line", "{n} lines", "{n} Buchung", "{n} Buchungen")`: one form for exactly one, the other
/// for every other count. The texts name the caller's variables.
#[macro_export]
macro_rules! tr_plural {
    ($n:expr, $en_one:literal, $en_many:literal, $de_one:literal, $de_many:literal $(,)?) => {
        if $n == 1 {
            $crate::tr!($en_one, $de_one)
        } else {
            $crate::tr!($en_many, $de_many)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_language_defaults_to_english_on_a_thread_without_an_override() {
        assert_eq!(with_lang(Language::En, lang), Language::En);
        assert_eq!(with_lang(Language::De, lang), Language::De);
    }

    #[test]
    fn with_lang_restores_the_previous_language_afterwards() {
        let outside = lang();
        with_lang(Language::De, || {});
        assert_eq!(lang(), outside);
    }

    #[test]
    fn with_lang_nests_and_each_level_restores_its_outer_language() {
        with_lang(Language::De, || {
            assert_eq!(lang(), Language::De);
            with_lang(Language::En, || assert_eq!(lang(), Language::En));
            assert_eq!(lang(), Language::De);
        });
    }

    #[test]
    fn with_lang_returns_the_value_of_the_closure() {
        assert_eq!(with_lang(Language::De, || 42), 42);
    }

    #[test]
    fn a_text_reads_in_the_current_language() {
        let text = Text::new("Undo", "Widerrufen");
        assert_eq!(with_lang(Language::En, || text.get()), "Undo");
        assert_eq!(with_lang(Language::De, || text.get()), "Widerrufen");
    }

    #[test]
    fn tr_formats_inline_names_in_the_current_language() {
        let n = 3;
        for (language, expected) in [(Language::En, "3 lines"), (Language::De, "3 Buchungen")] {
            let text = with_lang(language, || tr!("{n} lines", "{n} Buchungen"));
            assert_eq!(text, expected, "{language:?}");
        }
    }

    #[test]
    fn tr_formats_trailing_arguments_in_the_current_language() {
        for (language, expected) in [(Language::En, "Hello Ada"), (Language::De, "Hallo Ada")] {
            let text = with_lang(language, || tr!("Hello {}", "Hallo {}", "Ada"));
            assert_eq!(text, expected, "{language:?}");
        }
    }

    #[test]
    fn tr_without_arguments_gives_the_plain_text() {
        assert_eq!(
            with_lang(Language::De, || tr!("Undo", "Widerrufen")),
            "Widerrufen"
        );
        assert_eq!(
            with_lang(Language::En, || tr!("Undo", "Widerrufen")),
            "Undo"
        );
    }

    #[test]
    fn tr_plural_uses_the_singular_only_for_exactly_one() {
        for (n, en, de) in [
            (0, "0 lines", "0 Buchungen"),
            (1, "1 line", "1 Buchung"),
            (2, "2 lines", "2 Buchungen"),
        ] {
            let english = with_lang(Language::En, || {
                tr_plural!(n, "{n} line", "{n} lines", "{n} Buchung", "{n} Buchungen")
            });
            let german = with_lang(Language::De, || {
                tr_plural!(n, "{n} line", "{n} lines", "{n} Buchung", "{n} Buchungen")
            });
            assert_eq!(english, en, "{n}");
            assert_eq!(german, de, "{n}");
        }
    }
}
