/// Template variable resolution for watermark text.

#[derive(Default)]
pub struct TemplateContext {
    pub filename: String,
    pub ext: String,
    pub page: Option<u32>,
    pub total_pages: Option<u32>,
    pub counter: u32,
}

/// Resolve template variables in `template`, replacing `{var}` placeholders
/// with their computed values from `ctx` and the environment.
pub fn resolve(template: &str, ctx: &TemplateContext) -> String {
    let now = chrono::Local::now();

    let mut result = template.to_string();

    result = result.replace("{timestamp}", &now.format("%Y-%m-%dT%H:%M:%S").to_string());
    result = result.replace("{date}", &now.format("%Y-%m-%d").to_string());
    result = result.replace("{time}", &now.format("%H:%M:%S").to_string());
    result = result.replace("{filename}", &ctx.filename);
    result = result.replace("{ext}", &ctx.ext);

    let author = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    result = result.replace("{author}", &author);

    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    result = result.replace("{hostname}", &host);

    result = result.replace(
        "{page}",
        &ctx.page.map(|p| p.to_string()).unwrap_or_default(),
    );
    result = result.replace(
        "{total_pages}",
        &ctx.total_pages.map(|p| p.to_string()).unwrap_or_default(),
    );

    // Replace {uuid} – each occurrence gets a fresh UUID
    while result.contains("{uuid}") {
        result = result.replacen("{uuid}", &uuid::Uuid::new_v4().to_string(), 1);
    }

    result = result.replace("{counter}", &format!("{:03}", ctx.counter));

    result
}
