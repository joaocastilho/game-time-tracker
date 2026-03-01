use muda::Menu;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub fn setup_tray(menu: &Menu) -> Result<TrayIcon, anyhow::Error> {
    // Under Windows, index 1 usually refers to the first embedded icon (our main window icon).
    // The `tray-icon` crate looks it up via `LoadImageW(GetModuleHandleW(NULL), MAKEINTRESOURCEW(resource_id), ...)`
    let icon = match Icon::from_resource(1, Some((32, 32))) {
        Ok(embedded_icon) => embedded_icon,
        Err(e) => {
            log::warn!("Failed to load embedded icon (resource id 1): {}. Falling back to default generated icon.", e);
            let width = 32;
            let height = 32;
            let rgba = crate::icon::icon_rgba();
            Icon::from_rgba(rgba, width, height)?
        }
    };

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip("Game Time Tracker (0 active)")
        .with_icon(icon)
        .build()?;

    Ok(tray_icon)
}
