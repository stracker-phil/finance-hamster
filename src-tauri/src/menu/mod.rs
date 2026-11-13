//! The menu bar: macOS's standard items around the bus commands (ADR 0032). A menu item's id is its
//! command id, so choosing it runs the command.

use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Wry};

use crate::commands;

/// The app's own menus, each a list of groups of command ids with a separator between groups.
const MENUS: &[(&str, &[&[&str]])] = &[
    ("View", &[&["palette.open", "filter.focus"]]),
    (
        "Go",
        &[
            &[
                "go.today",
                "go.subscriptions",
                "go.statements",
                "go.documents",
            ],
            &["go.back", "go.forward"],
        ],
    ),
];

type Builder<'a> = SubmenuBuilder<'a, Wry, AppHandle>;

fn with_command<'a>(
    app: &'a AppHandle,
    builder: Builder<'a>,
    id: &str,
) -> tauri::Result<Builder<'a>> {
    let Some(command) = commands::find(id) else {
        log::warn!("The menu names an unknown command {id}");
        return Ok(builder);
    };
    let mut item = MenuItemBuilder::with_id(command.id, command.title);
    if let Some(accelerator) = command.accelerator {
        item = item.accelerator(accelerator);
    }
    Ok(builder.item(&item.build(app)?))
}

fn submenu(app: &AppHandle, title: &str, groups: &[&[&str]]) -> tauri::Result<Submenu<Wry>> {
    let mut builder = SubmenuBuilder::new(app, title);
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            builder = builder.separator();
        }
        for id in *group {
            builder = with_command(app, builder, id)?;
        }
    }
    builder.build()
}

fn app_menu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let about = AboutMetadata {
        name: Some(crate::APP_NAME.to_string()),
        ..AboutMetadata::default()
    };
    let builder = SubmenuBuilder::new(app, crate::APP_NAME)
        .about(Some(about))
        .separator();
    with_command(app, builder, "settings.open")?
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let app_menu = app_menu(app)?;
    let file = SubmenuBuilder::new(app, "File").close_window().build()?;
    let edit = with_command(app, SubmenuBuilder::new(app, "Edit"), "edit.undo")?;
    let edit = with_command(app, edit, "edit.redo")?
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    let mut menus: Vec<Submenu<Wry>> = vec![app_menu, file, edit];
    for (title, groups) in MENUS {
        menus.push(submenu(app, title, groups)?);
    }
    menus.push(window);
    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = menus.iter().map(|m| m as _).collect();
    Menu::with_items(app, &refs)
}

/// Runs the command a menu item names; macOS's own items never reach here.
pub fn on_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    if let Err(e) = commands::run(app, id) {
        log::warn!("Menu item {id} failed: {e}");
    }
}

#[cfg(test)]
mod tests;
