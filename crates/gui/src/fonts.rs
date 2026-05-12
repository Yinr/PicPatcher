pub fn install_fonts(ctx: &egui::Context) {
    let Some((name, bytes)) = load_cjk_font() else {
        return;
    };
    let name = name.to_string();

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert(name.clone(), egui::FontData::from_owned(bytes).into());

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, name.clone());
    }

    ctx.set_fonts(fonts);
}

fn load_cjk_font() -> Option<(&'static str, Vec<u8>)> {
    let candidates = cjk_font_candidates();
    for (name, path) in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            return Some((name, bytes));
        }
    }
    None
}

#[cfg(windows)]
fn cjk_font_candidates() -> Vec<(&'static str, std::path::PathBuf)> {
    let fonts_dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join("Fonts");

    [
        ("Microsoft YaHei", "msyh.ttc"),
        ("Microsoft YaHei UI", "msyh.ttc"),
        ("DengXian", "Deng.ttf"),
        ("SimHei", "simhei.ttf"),
        ("SimSun", "simsun.ttc"),
    ]
    .into_iter()
    .map(|(name, file)| (name, fonts_dir.join(file)))
    .collect()
}

#[cfg(not(windows))]
fn cjk_font_candidates() -> Vec<(&'static str, std::path::PathBuf)> {
    [
        (
            "Noto Sans CJK",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ),
        (
            "Noto Sans CJK",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        ),
        (
            "WenQuanYi Micro Hei",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ),
    ]
    .into_iter()
    .map(|(name, path)| (name, std::path::PathBuf::from(path)))
    .collect()
}
