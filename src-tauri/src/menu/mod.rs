//! The menu bar: macOS's standard items around the bus commands (ADR 0032). A menu item's id is its
//! command id, so choosing it runs the command.

use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Wry};

use crate::commands;
use crate::i18n::Text;
use crate::tr;

/// The app's own menus, each a list of groups of command ids with a separator between groups.
const MENUS: &[(Text, &[&[&str]])] = &[
    (
        Text::new("View", "Darstellung"),
        &[&["palette:open", "filter:focus"], &["view:payslips"]],
    ),
    (
        Text::new("Go", "Gehe zu"),
        &[
            &[
                "go:today",
                "go:subscriptions",
                "go:statements",
                "go:documents",
            ],
            &["go:back", "go:forward"],
        ],
    ),
];

const HELP: &[&[&str]] = &[&["app:logs", "app:data"]];

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
    let mut item = MenuItemBuilder::with_id(command.id, command.title.get());
    let overrides = commands::keys::load(&crate::config_dir());
    if let Some(accelerator) = commands::keys::effective(command, &overrides) {
        item = item.accelerator(accelerator);
    }
    Ok(builder.item(&item.build(app)?))
}

fn submenu(app: &AppHandle, title: Text, groups: &[&[&str]]) -> tauri::Result<Submenu<Wry>> {
    let mut builder = SubmenuBuilder::new(app, title.get());
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
    let name = crate::APP_NAME;
    let builder = SubmenuBuilder::new(app, name)
        .about_with_text(tr!("About {name}", "Über {name}"), Some(about))
        .separator();
    with_command(app, builder, "app:settings")?
        .separator()
        .services_with_text(tr!("Services", "Dienste"))
        .separator()
        .hide_with_text(tr!("Hide {name}", "{name} ausblenden"))
        .hide_others_with_text(tr!("Hide Others", "Andere ausblenden"))
        .show_all_with_text(tr!("Show All", "Alle einblenden"))
        .separator()
        .quit_with_text(tr!("Quit {name}", "{name} beenden"))
        .build()
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let app_menu = app_menu(app)?;
    let file = SubmenuBuilder::new(app, tr!("File", "Ablage"));
    let file = with_command(app, file, "statement:import")?;
    let file = with_command(app, file, "document:add")?;
    let file = with_command(app, file, "subscription:new")?
        .separator()
        .close_window_with_text(tr!("Close Window", "Fenster schließen"))
        .build()?;
    let edit = SubmenuBuilder::new(app, tr!("Edit", "Bearbeiten"));
    let edit = with_command(app, edit, "edit:undo")?;
    let edit = with_command(app, edit, "edit:redo")?
        .separator()
        .cut_with_text(tr!("Cut", "Ausschneiden"))
        .copy_with_text(tr!("Copy", "Kopieren"))
        .paste_with_text(tr!("Paste", "Einsetzen"))
        .select_all_with_text(tr!("Select All", "Alles auswählen"))
        .build()?;
    let window = SubmenuBuilder::new(app, tr!("Window", "Fenster"))
        .minimize_with_text(tr!("Minimize", "Im Dock ablegen"))
        .maximize_with_text(tr!("Zoom", "Zoomen"))
        .separator()
        .bring_all_to_front_with_text(tr!("Bring All to Front", "Alle nach vorne bringen"))
        .build()?;
    let mut menus: Vec<Submenu<Wry>> = vec![app_menu, file, edit];
    for (title, groups) in MENUS {
        menus.push(submenu(app, *title, groups)?);
    }
    menus.push(window);
    menus.push(submenu(app, Text::new("Help", "Hilfe"), HELP)?);
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

/// Builds the menu bar again, so changed shortcuts take effect.
pub fn refresh(app: &AppHandle) {
    match build(app).and_then(|menu| app.set_menu(menu)) {
        Ok(_) => {}
        Err(e) => log::warn!("Couldn't rebuild the menu bar: {e}"),
    }
}
