use std::collections::HashSet;

use super::{HELP, MENUS};
use crate::commands;

fn menu_ids() -> Vec<&'static str> {
    MENUS
        .iter()
        .flat_map(|(_, groups)| groups.iter().flat_map(|group| group.iter().copied()))
        .chain(HELP.iter().flat_map(|group| group.iter().copied()))
        .collect()
}

#[test]
fn every_menu_item_names_a_known_command() {
    for id in menu_ids() {
        assert!(commands::find(id).is_some(), "{id}");
    }
}

#[test]
fn no_command_appears_twice_across_the_menus() {
    let ids = menu_ids();
    let unique: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "{ids:?}");
}

#[test]
fn every_menu_title_has_an_english_and_a_german_text() {
    for (title, _) in MENUS {
        assert!(!title.en.is_empty(), "{title:?}");
        assert!(!title.de.is_empty(), "{title:?}");
    }
    let titles: Vec<(&str, &str)> = MENUS.iter().map(|(t, _)| (t.en, t.de)).collect();
    assert!(titles.contains(&("View", "Darstellung")), "{titles:?}");
    assert!(titles.contains(&("Go", "Gehe zu")), "{titles:?}");
}

#[test]
fn the_help_menu_offers_the_log_and_data_folders() {
    assert_eq!(HELP, &[&["app:logs", "app:data"][..]]);
}
