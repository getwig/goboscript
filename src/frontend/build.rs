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
    codegen::build::{
        build_impl,
        BuildOptions,
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
    let artifact = build_impl(fs, canonical_input, file.as_file_mut(), None, options)?;
    if !artifact.failure() {
        file.persist(&output)?;
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
