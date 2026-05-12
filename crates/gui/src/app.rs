use crate::editor::EditorState;
use crate::i18n::{texts, Language};
use crate::runner::RunnerState;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Tab {
    Editor,
    Runner,
}

pub struct PicPatcherApp {
    tab: Tab,
    language: Language,
    pub editor: EditorState,
    pub runner: RunnerState,
}

impl PicPatcherApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            tab: Tab::Editor,
            language: Language::detect(),
            editor: EditorState::default(),
            runner: RunnerState::default(),
        }
    }
}

impl eframe::App for PicPatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = texts(self.language);

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("PicPatcher");
                ui.separator();
                ui.selectable_value(&mut self.tab, Tab::Editor, t.tab_editor);
                ui.selectable_value(&mut self.tab, Tab::Runner, t.tab_runner);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.language,
                            Language::ZhCn,
                            Language::ZhCn.short_label(),
                        );
                        ui.selectable_value(
                            &mut self.language,
                            Language::En,
                            Language::En.short_label(),
                        );
                    });
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Editor => self.editor.ui(ui, ctx, t),
            Tab::Runner => self.runner.ui(ui, ctx, t),
        });
    }
}
