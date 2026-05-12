use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use picpatcher_core::{
    batch::{BatchOptions, BatchProgress, BatchSummary, OutputMode},
    parse_extensions, plan_batch, run_batch, Config, DEFAULT_EXTENSIONS,
};

#[derive(Parser, Debug)]
#[command(name = "picpatcher", version, about = "Batch image overlay patcher")]
struct Args {
    /// Path to the JSON config file (created by the GUI).
    #[arg(short, long)]
    config: PathBuf,

    /// Input directory containing images.
    #[arg(short, long)]
    input: PathBuf,

    /// Comma-separated list of extensions to process.
    #[arg(short = 'e', long)]
    ext: Option<String>,

    /// Recurse into subdirectories.
    #[arg(short, long)]
    recursive: bool,

    /// Overwrite originals (destructive). If unset, output goes to `<input>/_modified/`.
    #[arg(long)]
    in_place: bool,

    /// Custom sibling output directory name (only used without --in-place).
    #[arg(long, default_value = "_modified")]
    out_dir: String,

    /// Print planned input/output paths without writing any files.
    #[arg(long)]
    dry_run: bool,
}

struct CliProgress {
    bar: Mutex<ProgressBar>,
}

impl BatchProgress for CliProgress {
    fn on_start(&self, total: usize) {
        let bar = self.bar.lock().unwrap();
        bar.set_length(total as u64);
        bar.set_position(0);
    }
    fn on_item(&self, done: usize, _total: usize, path: &Path, error: Option<&str>) {
        let bar = self.bar.lock().unwrap();
        if let Some(e) = error {
            bar.println(format!("ERR {}: {}", path.display(), e));
        }
        bar.set_position(done as u64);
        bar.set_message(
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
        );
    }
    fn on_finish(&self, summary: &BatchSummary) {
        let bar = self.bar.lock().unwrap();
        bar.finish_with_message(format!(
            "done: {} ok / {} fail / {} total",
            summary.succeeded, summary.failed, summary.total
        ));
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let cfg = Config::load(&args.config)?;
    let extensions = args
        .ext
        .as_deref()
        .map(parse_extensions)
        .unwrap_or_else(BatchOptions::default_extensions);

    let opts = BatchOptions {
        input_dir: args.input.clone(),
        extensions,
        recursive: args.recursive,
        output: if args.in_place {
            OutputMode::InPlace
        } else {
            OutputMode::Sibling {
                dir_name: args.out_dir.clone(),
            }
        },
    };

    if args.dry_run {
        let overlay_path = cfg.resolve_overlay(&args.config);
        println!("config: {}", args.config.display());
        println!("overlay: {}", overlay_path.display());
        println!("input: {}", opts.input_dir.display());
        println!("extensions: {}", opts.extensions.join(","));
        println!("default extensions: {}", DEFAULT_EXTENSIONS.join(","));
        println!("mode: {}", if args.in_place { "in-place" } else { "copy" });
        let plan = plan_batch(&opts);
        println!("planned files: {}", plan.len());
        for item in plan {
            println!("{} -> {}", item.input.display(), item.output.display());
        }
        return Ok(());
    }

    let bar = ProgressBar::new(0);
    bar.set_style(ProgressStyle::with_template("{wide_bar} {pos}/{len} {msg}").unwrap());
    let progress = Arc::new(CliProgress {
        bar: Mutex::new(bar),
    });

    let summary = run_batch(&cfg, &args.config, &opts, progress)?;
    if summary.failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}
