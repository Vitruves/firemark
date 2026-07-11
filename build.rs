fn main() {
    // These messages are displayed during `cargo install firemark`.
    // Update them with each release.
    println!("cargo:warning=");
    println!("cargo:warning=firemark v0.1.4 — Release Notes");
    println!("cargo:warning=─────────────────────────────────");
    println!("cargo:warning=  New:");
    println!("cargo:warning=  • Entangled watermarking: marks are blended into salient content (text, edges),");
    println!(
        "cargo:warning=    so AI removal must reconstruct real detail and becomes visibly lossy"
    );
    println!("cargo:warning=  • Saliency-biased placement: watermarks target high-detail regions automatically");
    println!("cargo:warning=  • Anti-AI stroke entanglement: thin wavy strokes woven through dense text bands");
    println!("cargo:warning=    Part of anti-AI hardening, disable with --no-anti-ai");
    println!("cargo:warning=  • Copy-paste poisoning for PDFs: invisible scrambled text prevents clean text extraction");
    println!("cargo:warning=    On by default, disable with --no-copy-poison");
    println!(
        "cargo:warning=  • PDF-to-image conversion: -o output.png/jpeg now rasterizes via pdftoppm"
    );
    println!("cargo:warning=    Multi-page PDFs produce output_page1.png, output_page2.png, etc.");
    println!("cargo:warning=  Fixed:");
    println!("cargo:warning=  • --opacity now correctly affects PDF watermarks, anti-AI text, and filigrane patterns");
    println!("cargo:warning=  • Anti-AI text visibility scales smoothly with opacity (sqrt curve for readability)");
    println!("cargo:warning=");
    println!("cargo:warning=  Run `firemark --help` to get started.");
    println!("cargo:warning=  GitHub: https://github.com/Vitruves/firemark");
    println!("cargo:warning=");
}
