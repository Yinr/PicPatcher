use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use picpatcher_core::{
    batch::{BatchOptions, BatchProgress, BatchSummary, OutputMode},
    parse_extensions, run_batch, Config, DEFAULT_EXTENSIONS,
};

use crate::i18n::Texts;

pub struct RunState {
    pub running: AtomicBool,
    pub done: AtomicUsize,
    pub total: AtomicUsize,
    pub current: Mutex<String>,
    pub log: Mutex<Vec<String>>,
    pub summary: Mutex<Option<BatchSummary>>,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            running: AtomicBool::new(false),
            done: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
            current: Mutex::new(String::new()),
            log: Mutex::new(Vec::new()),
            summary: Mutex::new(None),
        }
    }
}

impl BatchProgress for RunState {
    fn on_start(&self, total: usize) {
        self.total.store(total, Ordering::SeqCst);
        self.done.store(0, Ordering::SeqCst);
        self.log.lock().unwrap().clear();
        *self.summary.lock().unwrap() = None;
    }
    fn on_item(&self, done: usize, _total: usize, path: &Path, error: Option<&str>) {
        self.done.store(done, Ordering::SeqCst);
        *self.current.lock().unwrap() = path.display().to_string();
        if let Some(e) = error {
            self.log
                .lock()
                .unwrap()
                .push(format!("ERR {}: {}", path.display(), e));
        }
    }
    fn on_finish(&self, summary: &BatchSummary) {
        *self.summary.lock().unwrap() = Some(summary.clone());
        self.running.store(false, Ordering::SeqCst);
    }
}

pub struct RunnerState {
    pub config_path: Option<PathBuf>,
    pub input_dir: Option<PathBuf>,
    pub extensions: String,
    pub recursive: bool,
    pub in_place: bool,
    pending_in_place: bool,
    pub out_dir_name: String,
    pub state: Arc<RunState>,
    pub status: String,
    confirm_in_place: bool,
}

impl Default for RunnerState {
    fn default() -> Self {
        Self {
            config_path: None,
            input_dir: None,
            extensions: DEFAULT_EXTENSIONS.join(","),
            recursive: true,
            in_place: false,
            pending_in_place: false,
            out_dir_name: "_modified".into(),
            state: Arc::new(RunState::default()),
            status: String::new(),
            confirm_in_place: false,
        }
    }
}

impl RunnerState {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, t: &Texts) {
        ui.horizontal(|ui| {
            if ui.button(t.choose_config).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    self.config_path = Some(p);
                }
            }
            if let Some(p) = &self.config_path {
                ui.label(p.display().to_string());
            } else {
                ui.label(t.no_config);
            }
        });
        ui.horizontal(|ui| {
            if ui.button(t.choose_input_dir).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.input_dir = Some(p);
                }
            }
            if let Some(p) = &self.input_dir {
                ui.label(p.display().to_string());
            } else {
                ui.label(t.no_input_dir);
            }
        });

        ui.horizontal(|ui| {
            ui.label(t.extensions);
            ui.text_edit_singleline(&mut self.extensions);
            ui.checkbox(&mut self.recursive, t.recursive);
        });
        ui.horizontal(|ui| {
            let mut requested_in_place = self.in_place;
            if ui
                .checkbox(&mut requested_in_place, t.overwrite_originals)
                .changed()
            {
                if requested_in_place {
                    self.pending_in_place = true;
                    self.confirm_in_place = true;
                } else {
                    self.in_place = false;
                    self.pending_in_place = false;
                    self.confirm_in_place = false;
                }
            }
            ui.add_enabled(
                !self.in_place,
                egui::TextEdit::singleline(&mut self.out_dir_name).hint_text(t.output_dir_name),
            );
        });

        ui.separator();

        let running = self.state.running.load(Ordering::SeqCst);
        let total = self.state.total.load(Ordering::SeqCst);
        let done = self.state.done.load(Ordering::SeqCst);

        ui.horizontal(|ui| {
            let can_run = !running && self.config_path.is_some() && self.input_dir.is_some();
            if ui
                .add_enabled(
                    can_run,
                    egui::Button::new(if running { t.running } else { t.run }),
                )
                .clicked()
            {
                self.start_run(ctx.clone(), t);
            }
            if total > 0 {
                let frac = if total > 0 {
                    done as f32 / total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(frac).text(format!("{done}/{total}")));
            }
        });

        if self.confirm_in_place {
            egui::Window::new(t.confirm_in_place_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(t.confirm_in_place_message);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t.cancel).clicked() {
                            self.pending_in_place = false;
                            self.confirm_in_place = false;
                        }
                        if ui.button(t.confirm_in_place_continue).clicked() {
                            if self.pending_in_place {
                                self.in_place = true;
                                self.pending_in_place = false;
                            }
                            self.confirm_in_place = false;
                        }
                    });
                });
        }

        if let Ok(cur) = self.state.current.lock() {
            if !cur.is_empty() && running {
                ui.label(format!("{}: {}", t.current, *cur));
            }
        }

        if let Some(summary) = self.state.summary.lock().unwrap().as_ref() {
            ui.colored_label(
                if summary.failed == 0 {
                    egui::Color32::LIGHT_GREEN
                } else {
                    egui::Color32::LIGHT_RED
                },
                format!(
                    "{}: {} {} / {} {} / {} {}",
                    t.finished,
                    summary.succeeded,
                    t.ok,
                    summary.failed,
                    t.fail,
                    summary.total,
                    t.total
                ),
            );
        }
        if !self.status.is_empty() {
            ui.label(&self.status);
        }

        ui.separator();
        ui.label(t.log);
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Ok(log) = self.state.log.lock() {
                for line in log.iter() {
                    ui.label(line);
                }
            }
        });

        if running {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn start_run(&mut self, ctx: egui::Context, t: &Texts) {
        let (Some(cfg_path), Some(input_dir)) = (self.config_path.clone(), self.input_dir.clone())
        else {
            return;
        };
        let cfg = match Config::load(&cfg_path) {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("{}: {e:#}", t.config_load_failed);
                return;
            }
        };
        let extensions = parse_extensions(&self.extensions);
        let opts = BatchOptions {
            input_dir,
            extensions,
            recursive: self.recursive,
            output: if self.in_place {
                OutputMode::InPlace
            } else {
                OutputMode::Sibling {
                    dir_name: self.out_dir_name.clone(),
                }
            },
        };
        let state = self.state.clone();
        let fatal = t.fatal;
        state.running.store(true, Ordering::SeqCst);
        thread::spawn(move || {
            let progress: Arc<dyn BatchProgress> = state.clone();
            if let Err(e) = run_batch(&cfg, &cfg_path, &opts, progress) {
                state.log.lock().unwrap().push(format!("{fatal}: {e:#}"));
                state.running.store(false, Ordering::SeqCst);
            }
            ctx.request_repaint();
        });
    }
}
