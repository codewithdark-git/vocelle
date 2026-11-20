use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

const MENU_PASTE_LAST: &str = "paste-last";
const MENU_SETTINGS: &str = "settings";
const MENU_QUIT: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let paste_last = MenuItem::with_id(
        app,
        MENU_PASTE_LAST,
        "Paste last dictation",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Vocelle", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &paste_last,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Vocelle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            // Clicking the menu takes focus from the app the user was in, so
            // this usually copies the text and asks for Ctrl+V.
            MENU_PASTE_LAST => crate::dictation::paste_last(app),
            MENU_SETTINGS => crate::show_main_window(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click opens settings; right-click shows the menu.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
