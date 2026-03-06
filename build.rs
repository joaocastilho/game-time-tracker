#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "Game Time Tracker");
    res.set(
        "FileDescription",
        "Lightweight background game time tracker",
    );
    res.set("CompanyName", "Joao Castilho");
    res.set("LegalCopyright", "Copyright (c) 2026 Joao Castilho");
    res.set("ProductVersion", "0.1.0");
    if let Err(e) = res.compile() {
        println!("cargo:warning=Failed to compile Windows resource: {}", e);
    }
}

#[cfg(not(windows))]
fn main() {}
