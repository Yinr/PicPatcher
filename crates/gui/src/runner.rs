use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use picpatcher_core::{
    batch::{BatchOptions, BatchProgress, BatchSummary, OutputMode},
    parse_extensions, run_batch, Config, DEFAULT_EXTENSIONS,
};

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
    pub out_dir_name: String,
    pub state: Arc<RunState>,
    pub status: String,
}

impl Default for RunnerState {
    fn default() -> Self {
        Self {
            config_path: None,
            input_dir: None,
            extensions: DEFAULT_EXTENSIONS.join(","),
            recursive: true,
            in_place: false,
            out_dir_name: "_modified".into(),
            state: Arc::new(RunState::default()),
            status: String::new(),
        }
    }
}

impl RunnerState {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if ui.button("Choose config…").clicked() {
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
                ui.label("(no config)");
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Choose input dir…").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.input_dir = Some(p);
                }
            }
            if let Some(p) = &self.input_dir {
                ui.label(p.display().to_string());
            } else {
                ui.label("(no input dir)");
            }
        });

        ui.horizontal(|ui| {
            ui.label("Extensions:");
            ui.text_edit_singleline(&mut self.extensions);
            ui.checkbox(&mut self.recursive, "Recursive");
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.in_place, "Overwrite originals (in-place)");
            ui.add_enabled(
                !self.in_place,
                egui::TextEdit::singleline(&mut self.out_dir_name).hint_text("output dir name"),
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
                    egui::Button::new(if running { "Running…" } else { "▶ Run" }),
                )
                .clicked()
            {
                if self.in_place {
                    // Soft confirm via status; destructive operation.
                    self.status = "Running in-place (originals will be overwritten).".into();
                }
                self.start_run(ctx.clone());
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

        if let Ok(cur) = self.state.current.lock() {
            if !cur.is_empty() && running {
                ui.label(format!("Current: {}", *cur));
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
                    "Finished: {} ok / {} fail / {} total",
                    summary.succeeded, summary.failed, summary.total
                ),
            );
        }
        if !self.status.is_empty() {
            ui.label(&self.status);
        }

        ui.separator();
        ui.label("Log:");
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

    fn start_run(&mut self, ctx: egui::Context) {
        let (Some(cfg_path), Some(input_dir)) = (self.config_path.clone(), self.input_dir.clone())
        else {
            return;
        };
        let cfg = match Config::load(&cfg_path) {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("config load failed: {e:#}");
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
        state.running.store(true, Ordering::SeqCst);
        thread::spawn(move || {
            let progress: Arc<dyn BatchProgress> = state.clone();
            if let Err(e) = run_batch(&cfg, &cfg_path, &opts, progress) {
                state.log.lock().unwrap().push(format!("FATAL: {e:#}"));
                state.running.store(false, Ordering::SeqCst);
            }
            ctx.request_repaint();
        });
    }
}
