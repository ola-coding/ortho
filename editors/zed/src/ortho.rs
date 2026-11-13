//! The Zed extension for ortho. Zed runs it as WebAssembly, and asks it how to start
//! the language server for SysML v2 files: `ortho lsp`, with the ortho the user's
//! `lsp.ortho.binary.path` setting names, else the one on the PATH, else the release
//! of ortho for the user's system, which the extension downloads from GitHub once and
//! keeps until a newer release comes out.
use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

/// Where ortho's releases are.
const REPOSITORY: &str = "ola-coding/ortho";

struct Ortho {
    /// The ortho downloaded, once found or fetched in this session.
    downloaded: Option<String>,
}

impl Ortho {
    /// The release of ortho for this system: downloaded if it is not there yet, and the
    /// releases before it removed. Offline, the one downloaded before will do.
    fn download(&mut self, server: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.downloaded
            && fs::metadata(path).is_ok_and(|meta| meta.is_file())
        {
            return Ok(path.clone());
        }
        let (os, arch) = zed::current_platform();
        let target = match (os, arch) {
            (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
            (zed::Os::Mac, _) => "x86_64-apple-darwin",
            (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-musl",
            (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-musl",
            // Windows on Arm runs the x64 build.
            (zed::Os::Windows, zed::Architecture::Aarch64 | zed::Architecture::X8664) => "x86_64-pc-windows-msvc",
            _ => return Err("ortho has no release for this system; install it with cargo install ortho".into()),
        };
        let windows = os == zed::Os::Windows;
        let binary = if windows { "ortho.exe" } else { "ortho" };

        zed::set_language_server_installation_status(server, &zed::LanguageServerInstallationStatus::CheckingForUpdate);
        let options = zed::GithubReleaseOptions { require_assets: true, pre_release: false };
        let release = match zed::latest_github_release(REPOSITORY, options) {
            Ok(release) => release,
            Err(error) => return self.downloaded_before(binary).ok_or(error),
        };
        // ortho-<target>.tar.gz holds ortho-<target>/ortho, as cargo binstall reads it.
        let archive = format!("ortho-{target}{}", if windows { ".zip" } else { ".tar.gz" });
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == archive)
            .ok_or_else(|| format!("ortho {} has no {archive}", release.version))?;
        let folder = format!("ortho-{}", release.version);
        let path = format!("{folder}/ortho-{target}/{binary}");
        if !fs::metadata(&path).is_ok_and(|meta| meta.is_file()) {
            zed::set_language_server_installation_status(server, &zed::LanguageServerInstallationStatus::Downloading);
            let kind = if windows { zed::DownloadedFileType::Zip } else { zed::DownloadedFileType::GzipTar };
            zed::download_file(&asset.download_url, &folder, kind)
                .map_err(|error| format!("could not download ortho {}: {error}", release.version))?;
            zed::make_file_executable(&path)?;
            for entry in fs::read_dir(".").map_err(|error| error.to_string())?.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("ortho-") && name != folder {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }
        zed::set_language_server_installation_status(server, &zed::LanguageServerInstallationStatus::None);
        self.downloaded = Some(path.clone());
        Ok(path)
    }

    /// The ortho a session before downloaded, if there is one.
    fn downloaded_before(&self, binary: &str) -> Option<String> {
        for release in fs::read_dir(".").ok()?.flatten() {
            let name = release.file_name().to_string_lossy().into_owned();
            if !name.starts_with("ortho-") {
                continue;
            }
            for target in fs::read_dir(release.path()).ok()?.flatten() {
                let target = target.file_name().to_string_lossy().into_owned();
                let path = format!("{name}/{target}/{binary}");
                if fs::metadata(&path).is_ok_and(|meta| meta.is_file()) {
                    return Some(path);
                }
            }
        }
        None
    }
}

impl zed::Extension for Ortho {
    fn new() -> Self {
        Ortho { downloaded: None }
    }

    fn language_server_command(&mut self, server: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
        let binary = LspSettings::for_worktree(server.as_ref(), worktree).ok().and_then(|settings| settings.binary);
        let args = binary.as_ref().and_then(|binary| binary.arguments.clone()).unwrap_or_else(|| vec!["lsp".into()]);
        let command = match binary.and_then(|binary| binary.path) {
            Some(path) => path,
            None => match worktree.which("ortho") {
                Some(path) => path,
                None => self.download(server)?,
            },
        };
        Ok(zed::Command { command, args, env: Default::default() })
    }

    fn language_server_initialization_options(
        &mut self,
        server: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(server.as_ref(), worktree).ok();
        Ok(settings.and_then(|settings| settings.initialization_options))
    }
}

zed::register_extension!(Ortho);
