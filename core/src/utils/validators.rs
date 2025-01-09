use std::str::FromStr;

use camino::Utf8PathBuf;

// ------------- PATH VALIDATORS -------------
pub enum PathKind {
    File,
    Dir,
}
/// Validates path kind & if it exists (optionally)
fn validate_path(
    raw_path: &str,
    path_kind: PathKind,
    allow_dne: bool,
) -> Result<Utf8PathBuf, String> {
    let path =
        Utf8PathBuf::from_str(raw_path).map_err(|_e| format!("Ill-formed path: {raw_path}"))?;

    let (allowed, err_msg) = match (path_kind, allow_dne) {
        (PathKind::File, false) => (
            path.is_file(),
            format!("Path `{path}` must be an existing file"),
        ),
        (PathKind::File, true) => (
            path.is_file() || !path.exists(),
            format!("Path `{path}` must be a file or a non-existent path"),
        ),
        (PathKind::Dir, false) => (
            path.is_dir(),
            format!("Path `{path}` must be an existing directory"),
        ),
        (PathKind::Dir, true) => (
            path.is_dir() || !path.exists(),
            format!("Path `{path}` must be a directory or a non-existent path"),
        ),
    };

    if allowed {
        Ok(path)
    } else {
        Err(err_msg)
    }
}

fn validate_file_with_extension(
    raw_path: &str,
    extensions: &[&str],
) -> Result<Utf8PathBuf, String> {
    let path = validate_path(raw_path, PathKind::File, false)?;
    let extension = format!(".{}", path.extension().unwrap_or_default());
    if !extensions.contains(&extension.as_str()) {
        Err(format!(
            "Path has incorrect extension, only {extensions:?} are permitted"
        ))
    } else {
        Ok(path)
    }
}

/// Validates path exists and is a file
pub(crate) fn validate_file(arg: &str) -> Result<Utf8PathBuf, String> {
    validate_path(arg, PathKind::File, false)
}
/// Validates file path exists and is either a json or yaml
pub(crate) fn validate_file_json_yaml(arg: &str) -> Result<Utf8PathBuf, String> {
    validate_file_with_extension(arg, &[".json", ".yml", ".yaml"])
}

/// Validates path is a file or does not exist
pub(crate) fn validate_file_allow_dne(arg: &str) -> Result<Utf8PathBuf, String> {
    validate_path(arg, PathKind::File, true)
}
/// Validates path exists and is a directory
pub(crate) fn validate_dir(arg: &str) -> Result<Utf8PathBuf, String> {
    validate_path(arg, PathKind::Dir, true)
}
/// Validates path is a directory or does not exist
pub(crate) fn validate_dir_allow_dne(arg: &str) -> Result<Utf8PathBuf, String> {
    validate_path(arg, PathKind::Dir, true)
}
