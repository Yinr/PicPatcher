use crate::editor::EditorState;
use crate::runner::RunnerState;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Tab {
    Editor,
    Runner,
}

pub struct PicPatcherApp {
    tab: Tab,
    pub editor: EditorState,
    pub runner: RunnerState,
}

impl PicPatcherApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            tab: Tab::Editor,
            editor: EditorState::default(),
            runner: RunnerState::default(),
        }
    }
}

impl eframe::App for PicPatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("PicPatcher");
                ui.separator();
                ui.selectable_value(&mut self.tab, Tab::Editor, "✏ Editor");
                ui.selectable_value(&mut self.tab, Tab::Runner, "▶ Runner");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Editor => self.editor.ui(ui, ctx),
            Tab::Runner => self.runner.ui(ui, ctx),
        });
    }
}
