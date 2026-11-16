use super::*;
use crate::error::ErrorKind;
use crate::profiles::{Profile, Profiles};
use crate::test_support::temp_config_dir;
use serde_json::json;

fn setup() -> (tempfile::TempDir, Profile) {
    let dir = temp_config_dir();
    let active = crate::profiles::open(dir.path(), "Demo").unwrap();
    let demo = crate::profiles::load(dir.path()).profiles.remove(0);
    assert_eq!(demo.id, active.id);
    (dir, demo)
}

fn request_add(name: &str, switch: bool) -> ProfileRequest {
    ProfileRequest::Add {
        name: name.into(),
        switch,
    }
}

fn request_switch(name: &str) -> ProfileRequest {
    ProfileRequest::Switch { name: name.into() }
}

fn request_rename(name: &str, to: &str) -> ProfileRequest {
    ProfileRequest::Rename {
        name: name.into(),
        to: to.into(),
    }
}

#[test]
fn list_answers_the_active_id_and_every_profile() {
    let (dir, demo) = setup();
    let alex = crate::profiles::add(dir.path(), "Alex").unwrap();

    let answer = answer(dir.path(), &ProfileRequest::List).unwrap();

    assert_eq!(
        answer,
        json!({
            "active": demo.id,
            "profiles": [
                { "id": demo.id, "name": "Demo" },
                { "id": alex.id, "name": "Alex" }
            ]
        })
    );
}

#[test]
fn switch_activates_the_profile_found_ignoring_case_and_spaces() {
    let (dir, demo) = setup();
    let alex = crate::profiles::add(dir.path(), "Alex").unwrap();

    let answer = answer(dir.path(), &request_switch("  aLEX ")).unwrap();

    assert_eq!(answer, json!({ "id": alex.id, "name": "Alex" }));
    assert_eq!(crate::profiles::load(dir.path()).active, alex.id);
    assert_ne!(alex.id, demo.id);
}

#[test]
fn switching_to_an_unknown_name_is_not_found_and_keeps_the_active_profile() {
    let (dir, demo) = setup();

    let err = answer(dir.path(), &request_switch("Casey")).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert_eq!(crate::profiles::load(dir.path()).active, demo.id);
}

#[test]
fn add_without_switch_makes_a_profile_with_its_own_folder_and_keeps_the_active_one() {
    let (dir, demo) = setup();

    let answer = answer(dir.path(), &request_add("Casey", false)).unwrap();

    let saved = crate::profiles::load(dir.path());
    let casey = saved.profiles[1].clone();
    assert_eq!(answer, json!({ "id": casey.id, "name": "Casey" }));
    assert_eq!(casey.name, "Casey");
    assert_eq!(saved.active, demo.id);
    assert!(crate::profiles::dir(dir.path(), &casey.id).is_dir());
}

#[test]
fn add_with_switch_makes_the_new_profile_active() {
    let (dir, _) = setup();

    let answer = answer(dir.path(), &request_add("Casey", true)).unwrap();

    let saved = crate::profiles::load(dir.path());
    assert_eq!(answer["id"].as_str(), Some(saved.active.as_str()));
    assert_eq!(answer["name"], "Casey");
    assert_eq!(saved.profiles.len(), 2);
}

#[test]
fn adding_a_taken_name_is_invalid_input_with_or_without_switch() {
    let (dir, _) = setup();

    for switch in [false, true] {
        let err = answer(dir.path(), &request_add(" demo ", switch)).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput, "{switch}");
    }
    assert_eq!(crate::profiles::load(dir.path()).profiles.len(), 1);
}

#[test]
fn rename_answers_the_profile_with_its_new_name() {
    let (dir, demo) = setup();

    let answer = answer(dir.path(), &request_rename("demo", "Casey")).unwrap();

    assert_eq!(answer, json!({ "id": demo.id, "name": "Casey" }));
    assert_eq!(
        crate::profiles::load(dir.path()),
        Profiles {
            active: demo.id.clone(),
            profiles: vec![Profile {
                id: demo.id,
                name: "Casey".into()
            }]
        }
    );
}

#[test]
fn renaming_an_unknown_profile_is_not_found() {
    let (dir, _) = setup();

    let err = answer(dir.path(), &request_rename("Casey", "Alex")).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
fn renaming_to_a_name_another_profile_has_is_invalid_input() {
    let (dir, _) = setup();
    crate::profiles::add(dir.path(), "Alex").unwrap();

    let err = answer(dir.path(), &request_rename("Demo", "ALEX")).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        crate::profiles::load(dir.path())
            .profiles
            .into_iter()
            .map(|p| p.name)
            .collect::<Vec<_>>(),
        ["Demo", "Alex"]
    );
}

#[test]
fn add_keeps_the_active_profile_and_named_finds_case_insensitively() {
    let (dir, demo) = setup();

    let casey = crate::profiles::add(dir.path(), "Casey").unwrap();

    let profiles = crate::profiles::load(dir.path());
    assert_eq!(profiles.active, demo.id);
    assert_eq!(crate::profiles::named(&profiles, " CASEY ").unwrap(), casey);
    assert_eq!(
        crate::profiles::named(&profiles, "Alex")
            .unwrap_err()
            .kind(),
        ErrorKind::NotFound
    );
}
