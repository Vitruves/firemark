fn main() {
    // These messages are displayed during `cargo install firemark`.
    // Update them with each release.
    println!("cargo:warning=");
    println!("cargo:warning=firemark v0.1.2 — Release Notes");
    println!("cargo:warning=─────────────────────────────────");
    println!("cargo:warning=  New:");
    println!("cargo:warning=  • Anti-AI adversarial prompt injection now ON by default (disable with --no-anti-ai)");
    println!("cargo:warning=  • Automatic update check against crates.io on startup");
    println!("cargo:warning=  • AI-removal hardening: universal post-render perturbation + per-renderer randomization");
    println!("cargo:warning=    Every render is now non-deterministic, making pixel-perfect AI removal impossible");
    println!("cargo:warning=");
    println!("cargo:warning=  Run `firemark --help` to get started.");
    println!("cargo:warning=  GitHub: https://github.com/Vitruves/firemark");
    println!("cargo:warning=");
}
