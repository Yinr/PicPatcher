#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod app;
mod editor;
mod fonts;
mod i18n;
mod runner;

use app::PicPatcherApp;

fn main() -> eframe::Result<()> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png"))
        .expect("embedded app icon must be a valid PNG");
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([800.0, 560.0])
            .with_title("PicPatcher")
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "PicPatcher",
        native_options,
        Box::new(|cc| {
            fonts::install_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(PicPatcherApp::new(cc)))
        }),
    )
}
