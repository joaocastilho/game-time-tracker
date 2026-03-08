use std::process::Command;

fn main() {
    println!("🚀 Running Local CI Checks...");

    let steps = [
        ("Formatting", "cargo", vec!["fmt", "--", "--check"]),
        ("Linting", "cargo", vec!["clippy", "--", "-D", "warnings"]),
        ("Testing", "cargo", vec!["test", "--all-targets"]),
    ];

    for (name, cmd, args) in steps {
        println!("\n--- {} ---", name);
        let status = Command::new(cmd)
            .args(&args)
            .status()
            .expect("Failed to execute command");

        if !status.success() {
            println!("\n❌ {} failed!", name);
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    println!("\n✅ All checks passed successfully!");
}
