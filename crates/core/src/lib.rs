pub mod batch;
pub mod compose;
pub mod config;

pub use batch::{
    collect_files, output_path_for, parse_extensions, plan_batch, run_batch, BatchOptions,
    BatchProgress, BatchSummary, OutputMode, PlannedFile, DEFAULT_EXTENSIONS,
};
pub use compose::{apply_to_image, load_overlay};
pub use config::{Config, CONFIG_VERSION};
