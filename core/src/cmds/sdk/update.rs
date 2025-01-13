use std::{fs, io::Write, process, str};

use camino::Utf8PathBuf;
use flate2::{write::GzEncoder, Compression};

use log::{debug, warn};
use sideko_rest_api::{
    models::{ApiVersion, VersionOrBump},
    resources::sdk::UpdateRequest,
    UploadFile,
};
use spinners::{Spinner, Spinners};
use tempfile::TempDir;

use crate::{
    result::{CliError, CliResult},
    styles::fmt_green,
    utils::get_sideko_client,
};

#[derive(clap::Args)]
pub struct SdkUpdateCommand {
    /// Path to SDK config
    #[arg(long, value_parser = crate::utils::validators::validate_file_yaml)]
    config: Utf8PathBuf,

    /// Path to root of SDK repo
    #[arg(long, value_parser = crate::utils::validators::validate_dir)]
    repo: Utf8PathBuf,

    /// Semantic version of generated SDK (e.g. `2.1.5`) or version bump (`patch`, `minor`, `major`, `rc`)
    #[arg(long)]
    version: String,

    /// API version to update SDK with (e.g. `2.1.5`)
    #[arg(long, default_value = "latest")]
    api_version: String,
}

impl SdkUpdateCommand {
    /// Validates:
    ///     - the path is an existing directory
    ///     - the path is a valid git repo root
    ///     - the git repo is clean (no un-committed files)
    ///
    /// Returns: the `.git` directory path within given path
    fn validate_git_root(&self) -> CliResult<Utf8PathBuf> {
        // validate .git is present
        let git_dir = self.repo.join(".git");
        if !(git_dir.is_dir() && git_dir.exists()) {
            return Err(CliError::general(format!(
                "Path is not the root of a git repository, {git_dir} not present"
            )));
        }

        // validate clean repo
        let status_output = process::Command::new("git")
            .current_dir(&self.repo)
            .args(["status", "--porcelain"])
            .output()
            .map_err(|e| {
                CliError::general_debug(
                    "Failed to check git status, is `git` installed?",
                    format!("{e:?}"),
                )
            })?;

        if !status_output.stdout.is_empty() {
            return Err(CliError::general_debug(
                "Git working directory is not clean. Please commit or stash your changes before updating",
                format!(
                    "`git status` failure (exit status {exit})\nstdout:\n{stdout}\nstderr:\n{stderr}",
                    exit = status_output.status,
                    stdout = str::from_utf8(&status_output.stdout).unwrap_or_default(),
                    stderr = str::from_utf8(&status_output.stderr).unwrap_or_default(),
                )
            ));
        }

        Ok(git_dir)
    }

    /// Validates the .sdk.json file in the root of the repo has an id field
    pub fn validate_sdk_id(&self) -> CliResult<String> {
        let md_path = self.repo.join(".sdk.json");
        if !(md_path.is_file() && md_path.exists()) {
            return Err(CliError::general_debug(
                "Could not determine SDK ID of the repository. Is this a Sideko SDK?",
                format!("SDK metadata path does not exist in repo: {md_path}"),
            ));
        }

        let md_str = fs::read_to_string(&md_path).map_err(|e| {
            CliError::general_debug(
                "Could not determine SDK ID of the repository. Is this a Sideko SDK?",
                format!("Unable to read SDK metadata path to string {md_path}: {e:?}"),
            )
        })?;
        debug!("Found sdk metadata: {md_str}");

        let md: SdkMetadata = serde_json::from_str(&md_str).map_err(|e| {
            CliError::general_debug(
                "Could not determine SDK ID of the repository. Is this a Sideko SDK?",
                format!("Unable to deserialize SDK metadata path to string {md_path}: {e:?}"),
            )
        })?;
        Ok(md.id)
    }

    pub async fn handle(&self) -> CliResult<()> {
        // validate and prep args
        let git_root = self.validate_git_root()?;
        let prev_sdk_id = self.validate_sdk_id()?;
        let config = UploadFile::from_path(self.config.as_str()).map_err(|e| {
            CliError::io_custom(
                format!("Failed reading config from path: {}", &self.config),
                e,
            )
        })?;

        // Create a temporary directory for the tarred .git contents
        let temp_dir = TempDir::new()
            .map_err(|e| CliError::io_custom("Failed creating temporary directory", e))?;
        debug!("Created temp directory {:?}", temp_dir.path());

        // tar .git in prep for update request
        let archive_into = temp_dir.path().join("git.tar.gz");
        debug!("Tarring .git into {archive_into:?}...");
        let mut tar_gz = fs::File::create(&archive_into)?;
        let encoder = GzEncoder::new(&tar_gz, Compression::default());
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(".", git_root)?;
        tar.into_inner()?.finish()?;
        tar_gz.flush()?;
        let prev_sdk_git = UploadFile::from_path(&archive_into.to_string_lossy())?;
        debug!("Tar complete: {} bytes", prev_sdk_git.content.len());

        let mut client = get_sideko_client();

        let start = chrono::Utc::now();
        let mut sp = Spinner::new(Spinners::Circle, "🪄  Updating SDK...".into());
        let patch_content = client
            .sdk()
            .update(UpdateRequest {
                api_version: Some(ApiVersion::Str(self.api_version.clone())),
                config,
                prev_sdk_git,
                prev_sdk_id,
                sdk_version: VersionOrBump::Str(self.version.clone()),
            })
            .await?;

        debug!(
            "Update generation took {}s",
            (chrono::Utc::now() - start).num_seconds()
        );

        if patch_content.is_empty() {
            sp.stop();
            warn!("No updates to apply");
            return Ok(());
        }

        // write and apply git patch
        let patch_filename = "sdk_update.patch";
        let patch_path = self.repo.join(patch_filename);
        fs::write(&patch_path, &patch_content)
            .map_err(|e| CliError::io_custom("Failed writing sdk git patch file", e))?;

        let patch_output = process::Command::new("git")
            .current_dir(&self.repo)
            .arg("apply")
            .arg(patch_filename)
            .output()
            .map_err(|e| {
                CliError::general_debug(
                    "Failed to run git patch, is `git` installed?",
                    format!("{e:?}"),
                )
            })?;

        if patch_output.status.success() {
            sp.stop_and_persist(&fmt_green("✔"), "🚀 Update applied!".into());
            fs::remove_file(&patch_path)?;
            Ok(())
        } else {
            sp.stop();
            Err(CliError::general_debug(
                "Failed to apply update",
                format!(
                    "`git patch` failure (exit status {exit})\nstdout:\n{stdout}\nstderr:\n{stderr}",
                    exit = patch_output.status,
                    stdout = str::from_utf8(&patch_output.stdout).unwrap_or_default(),
                    stderr = str::from_utf8(&patch_output.stderr).unwrap_or_default(),
                ),
            ))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct SdkMetadata {
    pub id: String,
}
