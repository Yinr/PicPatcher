use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::compose::{load_overlay, process_file};
use crate::config::Config;

pub const DEFAULT_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

#[derive(Debug, Clone)]
pub enum OutputMode {
    /// Write into a sibling directory under the input root.
    Sibling { dir_name: String },
    /// Overwrite originals in-place.
    InPlace,
}

impl Default for OutputMode {
    fn default() -> Self {
        Self::Sibling {
            dir_name: "_modified".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchOptions {
    pub input_dir: PathBuf,
    pub extensions: Vec<String>, // lowercase, no dot
    pub recursive: bool,
    pub output: OutputMode,
}

impl BatchOptions {
    pub fn default_extensions() -> Vec<String> {
        DEFAULT_EXTENSIONS
            .iter()
            .copied()
            .map(|s| s.to_string())
            .collect()
    }
}

pub fn parse_extensions(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().trim_start_matches('.').to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Debug, Default, Clone)]
pub struct BatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<(PathBuf, String)>,
}

pub trait BatchProgress: Send + Sync {
    fn on_start(&self, total: usize);
    fn on_item(&self, done: usize, total: usize, path: &Path, error: Option<&str>);
    fn on_finish(&self, summary: &BatchSummary);
}

pub struct NoopProgress;
impl BatchProgress for NoopProgress {
    fn on_start(&self, _: usize) {}
    fn on_item(&self, _: usize, _: usize, _: &Path, _: Option<&str>) {}
    fn on_finish(&self, _: &BatchSummary) {}
}

pub fn collect_files(opts: &BatchOptions) -> Vec<PathBuf> {
    let max_depth = if opts.recursive { usize::MAX } else { 1 };
    WalkDir::new(&opts.input_dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| {
                    opts.extensions
                        .iter()
                        .any(|ext| ext.eq_ignore_ascii_case(e))
                })
                .unwrap_or(false)
        })
        .collect()
}

pub fn output_path_for(input: &Path, opts: &BatchOptions) -> PathBuf {
    match &opts.output {
        OutputMode::InPlace => input.to_path_buf(),
        OutputMode::Sibling { dir_name } => {
            // Mirror structure under <input_dir>/<dir_name>/...
            let rel = input
                .strip_prefix(&opts.input_dir)
                .unwrap_or(input)
                .to_path_buf();
            opts.input_dir.join(dir_name).join(rel)
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub input: PathBuf,
    pub output: PathBuf,
}

pub fn plan_batch(opts: &BatchOptions) -> Vec<PlannedFile> {
    collect_files(opts)
        .into_iter()
        .map(|input| PlannedFile {
            output: output_path_for(&input, opts),
            input,
        })
        .collect()
}

pub fn run_batch(
    cfg: &Config,
    config_path: &Path,
    opts: &BatchOptions,
    progress: Arc<dyn BatchProgress>,
) -> Result<BatchSummary> {
    let overlay_path = cfg.resolve_overlay(config_path);
    let overlay = load_overlay(&overlay_path)
        .with_context(|| format!("loading overlay {}", overlay_path.display()))?;

    let plan = plan_batch(opts);
    let total = plan.len();
    progress.on_start(total);

    let counter = AtomicUsize::new(0);
    let results: Vec<(PathBuf, Result<()>)> = plan
        .par_iter()
        .map(|item| {
            let res = process_file(&item.input, &item.output, &overlay, cfg.x, cfg.y);
            let done = counter.fetch_add(1, Ordering::SeqCst) + 1;
            let err_msg = res.as_ref().err().map(|e| format!("{e:#}"));
            progress.on_item(done, total, &item.input, err_msg.as_deref());
            (item.input.clone(), res)
        })
        .collect();

    let mut summary = BatchSummary {
        total,
        ..Default::default()
    };
    for (p, r) in results {
        match r {
            Ok(()) => summary.succeeded += 1,
            Err(e) => {
                summary.failed += 1;
                summary.errors.push((p, format!("{e:#}")));
            }
        }
    }
    progress.on_finish(&summary);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extensions_normalizes_values() {
        assert_eq!(
            parse_extensions(" jpg, .PNG,,WebP "),
            vec!["jpg", "png", "webp"]
        );
    }

    #[test]
    fn output_path_mirrors_relative_structure() {
        let opts = BatchOptions {
            input_dir: PathBuf::from("C:/photos"),
            extensions: BatchOptions::default_extensions(),
            recursive: true,
            output: OutputMode::Sibling {
                dir_name: "_modified".to_string(),
            },
        };

        assert_eq!(
            output_path_for(Path::new("C:/photos/a/b/image.jpg"), &opts),
            PathBuf::from("C:/photos/_modified/a/b/image.jpg")
        );
    }

    #[test]
    fn output_path_in_place_returns_input_path() {
        let opts = BatchOptions {
            input_dir: PathBuf::from("C:/photos"),
            extensions: BatchOptions::default_extensions(),
            recursive: true,
            output: OutputMode::InPlace,
        };
        let input = Path::new("C:/photos/image.jpg");
        assert_eq!(output_path_for(input, &opts), input);
    }
}
