use std::path::PathBuf;

use annotate_snippets::renderer::{
    AnsiColor,
    Effects,
};
use clap::builder::Styles;
use clap_derive::{
    Parser,
    Subcommand,
};

const CLAP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Debug, Parser)]
#[command(
    version = option_env!("GIT_HASH").unwrap_or(env!("CARGO_PKG_VERSION")),
    about = "goboscript is the Scratch compiler.",
    long_about = "goboscript is the Scratch compiler.",
)]
#[command(styles=CLAP_STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compile a goboscript project to `.sb3`
    #[command(alias = "b")]
    Build {
        #[arg()]
        /// Project directory, if not given, the current directory is used.
        input: Option<PathBuf>,
        #[arg(short, long)]
        /// Output file, if not given, it will be the project directory's name + `.sb3`
        output: Option<PathBuf>,
        /// Report `local` variables in nowarp or recursive procedures as errors instead of
        /// warnings (they are shared by all calls of the procedure).
        #[arg(long)]
        deny_unsafe_locals: bool,
        /// Use the standard library in this directory instead of downloading it
        /// (`%include std/math` reads `<STD_PATH>/math.gs`). No network access.
        #[arg(long, env = "GOBOSCRIPT_STD_PATH")]
        std_path: Option<PathBuf>,
        /// Do not detect AI agents (no marker in the Stage notes, no AGENTS.md written into the
        /// project directory).
        #[arg(long)]
        no_ai_detection: bool,
        /// Do not add the Stage "notes" comment (goboscript logo, build info, notes file).
        #[arg(long)]
        no_stage_notes: bool,
        /// Do not add the Stage `_twconfig_` comment (TurboWarp settings, ignored by Scratch).
        #[arg(long)]
        no_turbowarp_config: bool,
        /// Also write a source map to this file: for every target, the `.gs` file, line and
        /// column of each block id.
        #[arg(long, value_name = "PATH")]
        debug_info: Option<PathBuf>,
        /// Derive block ids from the target, the enclosing script and the position in it, so
        /// editing one script does not renumber the blocks of other scripts.
        #[arg(long)]
        stable_ids: bool,
        /// Turn off constant folding, algebraic simplifications and dead-code removal: every
        /// operator is emitted as a block and unused procedures/functions are kept.
        #[arg(long)]
        no_fold: bool,
        /// Also write the parsed AST (before any compiler pass), with the source location of
        /// every span, to this file as JSON.
        #[arg(long, value_name = "PATH")]
        dump_ast: Option<PathBuf>,
    },

    /// Create a new goboscript project with a blank backdrop, a main sprite with a
    /// blank costume.
    #[command(alias = "n")]
    New {
        /// Name of directory to create new project, if not given, the current directory
        /// is used. If this is a path to an existing directory, it must be empty.
        #[arg()]
        name: Option<PathBuf>,

        /// Do not initialize a Git repository.
        #[arg(short = 'G', long)]
        no_git: bool,

        /// Version of the standard library to use. Defaults to bleeding-edge.
        #[arg(short = 's', long)]
        std: Option<String>,

        // (alias: --bmpres) Bitmap resolution. (default: 1)
        #[arg(short = 'b', long, alias = "bmpres")]
        bitmap_resolution: Option<u64>,

        /// (alias: --fps) Custom frame rate, used by TurboWarp.
        #[arg(short = 'f', long, alias = "fps")]
        frame_rate: Option<u64>,

        /// (alias: --clones) Custom maximum number of clones allowed, used by TurboWarp.
        /// Use `--max-clones inf` for infinite clones.
        #[arg(short = 'c', long, alias = "clones")]
        max_clones: Option<f64>,

        /// (alias: --limitless) Disable miscellaneous limits, used by TurboWarp.
        #[arg(short = 'l', long, alias = "limitless")]
        no_miscellaneous_limits: bool,

        /// (alias: --offscreen) Disable sprite fencing, used by TurboWarp.
        #[arg(short = 'o', long, alias = "offscreen")]
        no_sprite_fencing: bool,

        /// (alias: --interpolate) Enable frame interpolation, used by TurboWarp.
        #[arg(short = 'i', long, alias = "interpolate")]
        frame_interpolation: bool,

        /// (alias: --hqpen) Enable high quality pen, used by TurboWarp.
        #[arg(short = 'q', long, alias = "hqpen")]
        high_quality_pen: bool,

        /// (alias: --width) Custom stage width, used by TurboWarp.
        #[arg(short = 'W', long, alias = "width")]
        stage_width: Option<u64>,

        /// (alias: --height) Custom stage height, used by TurboWarp.
        #[arg(short = 'H', long, alias = "height")]
        stage_height: Option<u64>,

        /// Generate a Makefile for building the project.
        #[arg(short = 'm', long)]
        makefile: bool,
    },

    /// Format a goboscript project.
    #[command(alias = "f")]
    Fmt {
        /// Project directory or file, if not given, the current directory is used.
        #[arg(short, long)]
        input: Option<PathBuf>,
    },

    /// Generate completions for a shell.
    #[command()]
    Completions {
        /// The shell to generate the completions for.
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}
