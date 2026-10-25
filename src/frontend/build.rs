use std::{
    cell::RefCell,
    env,
    path::{
        Path,
        PathBuf,
    },
    rc::Rc,
};

use crate::{
    codegen::{
        build::{
            build_impl,
            BuildOptions,
        },
        ast_dump::ast_dump,
        source_map::source_map,
    },
    diagnostic::Artifact,
    vfs::RealFS,
};

pub fn build(
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    options: &BuildOptions,
) -> anyhow::Result<Artifact> {
    let input = input.unwrap_or_else(|| env::current_dir().unwrap());
    let canonical_input = input.canonicalize()?;
    let project_name = canonical_input.file_name().unwrap().to_str().unwrap();
    let output = output.unwrap_or_else(|| input.join(format!("{project_name}.sb3")));
    let fs = Rc::new(RefCell::new(RealFS));
    // Write into a temporary file next to the output and move it into place only when the
    // build succeeds, so a failed build never truncates or replaces an existing output (GBS-008).
    // Renaming replaces a symlink at the output path instead of writing through it.
    let mut file = temporary_output(&output)?;
    let artifact = build_impl(fs, canonical_input.clone(), file.as_file_mut(), None, options)?;
    if !artifact.failure() {
        file.persist(&output)?;
        if let Some(debug_info) = &options.debug_info {
            let std_path = options
                .std_path
                .as_ref()
                .and_then(|std_path| std_path.canonicalize().ok());
            let map = source_map(&artifact, &canonical_input, std_path.as_deref(), &mut RealFS);
            std::fs::write(debug_info, serde_json::to_vec_pretty(&map)?)?;
        }
        if let Some(dump_ast) = &options.dump_ast {
            let std_path = options
                .std_path
                .as_ref()
                .and_then(|std_path| std_path.canonicalize().ok());
            let dump = ast_dump(&artifact, &canonical_input, std_path.as_deref(), &mut RealFS);
            std::fs::write(dump_ast, serde_json::to_vec(&dump)?)?;
        }
    }
    Ok(artifact)
}

fn temporary_output(output: &Path) -> std::io::Result<tempfile::NamedTempFile> {
    let dir = match output.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut builder = tempfile::Builder::new();
    builder.prefix(".goboscript-").suffix(".sb3.tmp");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o644));
    }
    builder.tempfile_in(dir)
}
