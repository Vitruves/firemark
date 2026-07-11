/// Parse a color string (hex #RRGGBB, #RRGGBBAA, or named CSS color)
pub fn parse_color(s: &str) -> Result<[u8; 4], String> {
    let color = csscolorparser::parse(s).map_err(|e| format!("Invalid color '{s}': {e}"))?;
    Ok([
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    ])
}

/// Parse an offset string like "10,-5"
pub fn parse_offset(s: &str) -> Result<(i32, i32), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid offset '{s}': expected 'x,y'"));
    }
    let x = parts[0]
        .trim()
        .parse::<i32>()
        .map_err(|e| format!("Invalid offset x: {e}"))?;
    let y = parts[1]
        .trim()
        .parse::<i32>()
        .map_err(|e| format!("Invalid offset y: {e}"))?;
    Ok((x, y))
}

/// Parse a page range string like "1,3-5,8" or "all"
pub fn parse_page_range(s: &str) -> Result<PageRange, String> {
    if s.eq_ignore_ascii_case("all") {
        return Ok(PageRange::All);
    }

    let mut pages = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() != 2 {
                return Err(format!("Invalid range: {part}"));
            }
            let start: u32 = bounds[0]
                .trim()
                .parse()
                .map_err(|_| format!("Invalid page number in range: {part}"))?;
            let end: u32 = bounds[1]
                .trim()
                .parse()
                .map_err(|_| format!("Invalid page number in range: {part}"))?;
            if start > end {
                return Err(format!("Invalid range {start}-{end}: start > end"));
            }
            for p in start..=end {
                pages.push(p);
            }
        } else {
            let p: u32 = part
                .parse()
                .map_err(|_| format!("Invalid page number: {part}"))?;
            pages.push(p);
        }
    }

    pages.sort();
    pages.dedup();
    Ok(PageRange::Pages(pages))
}

#[derive(Debug, Clone, Default)]
pub enum PageRange {
    #[default]
    All,
    Pages(Vec<u32>),
}

impl PageRange {
    pub fn contains(&self, page: u32) -> bool {
        match self {
            PageRange::All => true,
            PageRange::Pages(pages) => pages.contains(&page),
        }
    }
}
