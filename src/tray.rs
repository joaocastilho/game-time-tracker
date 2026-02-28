use muda::Menu;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub fn setup_tray(menu: &Menu) -> Result<TrayIcon, anyhow::Error> {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        rgba.push(0); // R
        rgba.push(128); // G
        rgba.push(255); // B
        rgba.push(255); // A
    }

    let icon = Icon::from_rgba(rgba, width, height)?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip("Game Time Tracker (0 active)")
        .with_icon(icon)
        .build()?;

    Ok(tray_icon)
}
