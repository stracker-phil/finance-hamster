use super::*;
use crate::test_support::temp_config_dir;

const DATA_FILES: [&str; 5] = [
    "hamster.sqlite",
    "hamster.sqlite-wal",
    "hamster.sqlite-shm",
    "journal.jsonl",
    "reminders.json",
];
const DATA_FOLDERS: [&str; 4] = ["docs", "statements", "snapshots", "rules"];

fn write_old_data(root: &Path) {
    for name in DATA_FILES {
        std::fs::write(root.join(name), name).unwrap();
    }
    for folder in DATA_FOLDERS {
        std::fs::create_dir_all(root.join(folder)).unwrap();
        std::fs::write(root.join(folder).join("file.pdf"), folder).unwrap();
    }
    std::fs::write(root.join("settings.json"), "{}").unwrap();
    std::fs::write(root.join("keybindings.json"), "{}").unwrap();
}

fn names(root: &Path) -> Vec<String> {
    load(root).profiles.into_iter().map(|p| p.name).collect()
}

#[test]
fn the_first_open_creates_the_first_profile_and_saves_it_as_active() {
    let dir = temp_config_dir();

    let active = open(dir.path(), "Demo").unwrap();

    let saved = load(dir.path());
    assert_eq!(saved.profiles.len(), 1);
    assert_eq!(saved.profiles[0].name, "Demo");
    assert_eq!(saved.active, saved.profiles[0].id);
    assert_eq!(active.id, saved.active);
    assert!(active.first);
    assert_eq!(active.dir, dir.path().join("profiles").join(&active.id));
    assert!(active.dir.is_dir());
}

#[test]
fn the_first_open_moves_the_old_data_into_the_profile_and_copies_the_settings() {
    let dir = temp_config_dir();
    let root = dir.path();
    write_old_data(root);

    let active = open(root, "Demo").unwrap();

    for name in DATA_FILES {
        assert!(!root.join(name).exists(), "{name} left in root");
        assert_eq!(
            std::fs::read_to_string(active.dir.join(name)).unwrap(),
            name
        );
    }
    for folder in DATA_FOLDERS {
        assert!(!root.join(folder).exists(), "{folder} left in root");
        assert_eq!(
            std::fs::read_to_string(active.dir.join(folder).join("file.pdf")).unwrap(),
            folder
        );
    }
    assert!(root.join("settings.json").exists());
    assert!(active.dir.join("settings.json").exists());
    assert!(root.join("keybindings.json").exists());
    assert!(!active.dir.join("keybindings.json").exists());
}

#[test]
fn opening_again_returns_the_same_profile_and_moves_nothing() {
    let dir = temp_config_dir();
    let root = dir.path();
    let first = open(root, "Demo").unwrap();
    write_old_data(root);

    let second = open(root, "Other").unwrap();

    assert_eq!(second.id, first.id);
    assert_eq!(second.dir, first.dir);
    assert!(second.first);
    assert_eq!(names(root), ["Demo"]);
    for name in DATA_FILES {
        assert!(root.join(name).exists(), "{name}");
        assert!(!second.dir.join(name).exists(), "{name}");
    }
}

#[test]
fn an_unknown_active_id_falls_back_to_the_first_profile() {
    let dir = temp_config_dir();
    let root = dir.path();
    let first = open(root, "Demo").unwrap();
    create(root, "Alex").unwrap();
    let mut profiles = load(root);
    profiles.active = "missing".into();
    file(root).save(&profiles).unwrap();

    let active = open(root, "Demo").unwrap();

    assert_eq!(active.id, first.id);
    assert!(active.first);
    assert_eq!(load(root).active, first.id);
}

#[test]
fn a_created_profile_has_an_empty_folder_and_is_opened_next_but_is_not_the_first() {
    let dir = temp_config_dir();
    let root = dir.path();
    let first = open(root, "Demo").unwrap();

    let created = create(root, "Alex").unwrap();

    assert_eq!(created.name, "Alex");
    let folder = super::dir(root, &created.id);
    assert!(folder.is_dir());
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
    assert_eq!(names(root), ["Demo", "Alex"]);
    let active = open(root, "Demo").unwrap();
    assert_eq!(active.id, created.id);
    assert_eq!(active.dir, folder);
    assert!(!active.first);
    assert_ne!(active.id, first.id);
}

#[test]
fn names_are_trimmed() {
    let dir = temp_config_dir();
    open(dir.path(), "Demo").unwrap();

    let created = create(dir.path(), "  Alex \t").unwrap();

    assert_eq!(created.name, "Alex");
    assert_eq!(names(dir.path()), ["Demo", "Alex"]);
}

#[test]
fn an_empty_or_blank_name_is_invalid_input() {
    let dir = temp_config_dir();
    let first = open(dir.path(), "Demo").unwrap();

    for name in ["", "   ", "\t\n"] {
        let created = create(dir.path(), name).unwrap_err();
        let renamed = rename(dir.path(), &first.id, name).unwrap_err();
        assert!(matches!(created, AppError::InvalidInput(_)), "{name:?}");
        assert!(matches!(renamed, AppError::InvalidInput(_)), "{name:?}");
    }
    assert_eq!(names(dir.path()), ["Demo"]);
}

#[test]
fn a_name_taken_by_another_profile_is_invalid_input_whatever_its_case() {
    let dir = temp_config_dir();
    open(dir.path(), "Demo").unwrap();
    let alex = create(dir.path(), "Alex").unwrap();

    for name in ["Demo", "demo", " DEMO "] {
        let created = create(dir.path(), name).unwrap_err();
        let renamed = rename(dir.path(), &alex.id, name).unwrap_err();
        assert!(matches!(created, AppError::InvalidInput(_)), "{name:?}");
        assert!(matches!(renamed, AppError::InvalidInput(_)), "{name:?}");
    }
    assert_eq!(names(dir.path()), ["Demo", "Alex"]);
}

#[test]
fn renaming_changes_the_name_and_returns_all_profiles() {
    let dir = temp_config_dir();
    let first = open(dir.path(), "Demo").unwrap();

    let profiles = rename(dir.path(), &first.id, " Sam ").unwrap();

    assert_eq!(
        profiles.profiles,
        [Profile {
            id: first.id.clone(),
            name: "Sam".into()
        }]
    );
    assert_eq!(load(dir.path()), profiles);
}

#[test]
fn a_profile_can_be_renamed_to_its_own_name_in_another_case() {
    let dir = temp_config_dir();
    let first = open(dir.path(), "Demo").unwrap();

    rename(dir.path(), &first.id, "DEMO").unwrap();

    assert_eq!(names(dir.path()), ["DEMO"]);
}

#[test]
fn renaming_an_unknown_profile_is_not_found() {
    let dir = temp_config_dir();
    open(dir.path(), "Demo").unwrap();

    let err = rename(dir.path(), "missing", "Alex").unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)));
}

#[test]
fn activating_an_unknown_profile_is_not_found_and_changes_nothing() {
    let dir = temp_config_dir();
    let first = open(dir.path(), "Demo").unwrap();

    let err = activate(dir.path(), "missing").unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)));
    assert_eq!(load(dir.path()).active, first.id);
}

#[test]
fn activating_a_known_profile_makes_it_the_active_one() {
    let dir = temp_config_dir();
    let first = open(dir.path(), "Demo").unwrap();
    create(dir.path(), "Alex").unwrap();

    activate(dir.path(), &first.id).unwrap();

    assert_eq!(load(dir.path()).active, first.id);
    assert_eq!(open(dir.path(), "Demo").unwrap().id, first.id);
}

#[test]
fn the_profile_folder_is_under_profiles_by_id() {
    assert_eq!(
        dir(Path::new("/app"), "abc"),
        Path::new("/app/profiles/abc")
    );
}

#[test]
fn loading_without_a_file_gives_no_profiles() {
    let dir = temp_config_dir();

    assert_eq!(load(dir.path()), Profiles::default());
}
