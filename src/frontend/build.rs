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
    // A missing output or a regular file: write a temporary file next to it and move it into
    // place only when the build succeeds, so a failed build never truncates or replaces an
    // existing output (GBS-008). Anything else (a device such as /dev/null, a FIFO) is written
    // directly after a successful build and never replaced (GBS-036); a symbolic link is refused,
    // as upstream does (it opens the output with O_NOFOLLOW).
    let kind = output_kind(&output)?;
    let mut temporary = None;
    let mut memory = std::io::Cursor::new(Vec::new());
    let artifact = if kind == OutputKind::Replaceable {
        let mut file = temporary_output(&output)?;
        let artifact = build_impl(fs, canonical_input.clone(), file.as_file_mut(), None, options)?;
        temporary = Some(file);
        artifact
    } else {
        build_impl(fs, canonical_input.clone(), &mut memory, None, options)?
    };
    if !artifact.failure() {
        match temporary {
            Some(file) => {
                file.persist(&output)?;
            }
            None => {
                let mut out = std::fs::OpenOptions::new().write(true).open(&output)?;
                std::io::Write::write_all(&mut out, memory.get_ref())?;
            }
        }
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

#[derive(PartialEq, Eq)]
enum OutputKind {
    /// Missing or a regular file: written via a temporary file and a rename.
    Replaceable,
    /// A device, FIFO or socket: written in place.
    Special,
}

fn output_kind(output: &Path) -> anyhow::Result<OutputKind> {
    match std::fs::symlink_metadata(output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(OutputKind::Replaceable),
        Err(error) => Err(error.into()),
        Ok(metadata) if metadata.file_type().is_symlink() => anyhow::bail!(
            "output {} is a symbolic link; refusing to write through or replace it",
            output.display()
        ),
        Ok(metadata) if metadata.is_file() => Ok(OutputKind::Replaceable),
        Ok(metadata) if metadata.is_dir() => {
            anyhow::bail!("output {} is a directory", output.display())
        }
        Ok(_) => Ok(OutputKind::Special),
    }
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
