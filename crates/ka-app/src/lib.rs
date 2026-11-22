//! The app service behind the Tauri commands. `src-tauri` only forwards: every rule
//! (which paths the UI may touch, which settings it may write, what a valid design is)
//! lives here and in `ka-core`, with tests.

pub mod compose;
pub mod decide;
pub mod error;
pub mod guide;
pub mod kbarch;
pub mod live;
pub mod packs;
pub mod patterns;
pub mod practice;
pub mod problems;
mod runs;
pub mod samples;
pub mod sandbox;
pub mod settings;
pub mod window;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ka_core::{Design, DesignIssue, Registry, registry, validate};
use ka_store::UserDb;
use serde::Serialize;
use serde_json::Value;

pub use error::{AppError, AppErrorCode};
pub use runs::{ProblemTest, RunControl, RunInfo, RunRequest, SANDBOX_SEED, breaking_point};
pub use sandbox::SandboxPreset;

pub type Result<T, E = AppError> = std::result::Result<T, E>;

/// The largest image export the app writes (review A7).
pub const MAX_EXPORT_BYTES: usize = 256 * 1024 * 1024;

/// What an image export is (M7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Svg,
    Png,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Svg => "svg",
            ExportFormat::Png => "png",
        }
    }
}
/// Where the app keeps its data. Debug builds use `-dev` folders (see `src-tauri`).
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub user_db: PathBuf,
    /// True in debug builds: the UI shows a DEV DATA badge.
    pub dev_data: bool,
    /// The bundled problem catalogue (`content.db`, read-only). None or missing: the app
    /// runs without problems.
    pub content_db: Option<PathBuf>,
    /// Where installed problem packs live (`<data>/packs`). None: packs are off.
    pub packs_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AppInfo {
    pub version: String,
    pub dev_data: bool,
}

/// A recent file, and whether it is still on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RecentEntry {
    pub path: String,
    pub name: String,
    pub opened_at: i64,
    pub exists: bool,
}

/// A design read from disk, with the path it came from.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct OpenedDesign {
    pub path: String,
    pub design: Design,
}

pub struct App {
    user: UserDb,
    registry: Registry,
    info: AppInfo,
    /// Paths the user picked in a dialog (or saved to) this session: the UI may save to
    /// these without a dialog. Opening by path is limited to the recent-files list.
    session_paths: Mutex<HashSet<String>>,
    /// The problem catalogue, if it was found and opened.
    content: Option<ka_store::ContentDb>,
    /// Installed problem packs (M6.4).
    packs: packs::Packs,
    /// The live run, if one is playing (one at a time), and the id of the next.
    live: Mutex<Option<(u32, live::LiveRun)>>,
    next_run: std::sync::atomic::AtomicU32,
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl App {
    pub async fn open(paths: AppPaths) -> Result<App> {
        let user = UserDb::open(&paths.user_db).await?;
        let content = match &paths.content_db {
            Some(p) if p.exists() => Some(ka_store::ContentDb::open(p).await?),
            _ => None,
        };
        let packs = packs::Packs::new(paths.packs_dir.clone());
        packs.load(&user.packs().await?).await?;
        Ok(App {
            user,
            content,
            packs,
            registry: registry(),
            info: AppInfo {
                version: env!("CARGO_PKG_VERSION").into(),
                dev_data: paths.dev_data,
            },
            session_paths: Mutex::new(HashSet::new()),
            live: Mutex::new(None),
            next_run: std::sync::atomic::AtomicU32::new(1),
        })
    }

    pub fn info(&self) -> AppInfo {
        self.info.clone()
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The problem catalogue, or a `content_missing` error.
    pub fn content(&self) -> Result<&ka_store::ContentDb> {
        self.content
            .as_ref()
            .ok_or_else(|| AppError::new(AppErrorCode::ContentMissing, "content.db was not found"))
    }

    pub async fn close(&self) {
        self.user.close().await;
        if let Some(c) = &self.content {
            c.close().await;
        }
        self.packs.close().await;
    }

    // --- Settings -------------------------------------------------------------------

    fn ui_key(key: &str) -> Result<()> {
        if settings::ui_may_access(key) {
            Ok(())
        } else {
            Err(AppError::new(
                AppErrorCode::SettingNotAllowed,
                format!("setting {key:?}"),
            ))
        }
    }

    /// A setting the UI may read (allow-listed).
    pub async fn get_setting(&self, key: &str) -> Result<Option<Value>> {
        Self::ui_key(key)?;
        Ok(self.user.get_setting(key).await?)
    }

    /// A setting the UI may write (allow-listed, size-capped).
    pub async fn set_setting(&self, key: &str, value: &Value) -> Result<()> {
        Self::ui_key(key)?;
        let size = serde_json::to_string(value).map_or(usize::MAX, |s| s.len());
        if size > settings::MAX_SETTING_BYTES {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                format!("setting {key:?} is too large"),
            ));
        }
        Ok(self.user.set_setting(key, value).await?)
    }

    /// Rust-only settings (window bounds), never reachable from the UI.
    pub async fn get_internal_setting(&self, key: &str) -> Result<Option<Value>> {
        Ok(self.user.get_setting(key).await?)
    }

    pub async fn set_internal_setting(&self, key: &str, value: &Value) -> Result<()> {
        Ok(self.user.set_setting(key, value).await?)
    }

    // --- Recent files ---------------------------------------------------------------

    pub async fn recent_files(&self) -> Result<Vec<RecentEntry>> {
        Ok(self
            .user
            .recent_files()
            .await?
            .into_iter()
            .map(|r| RecentEntry {
                exists: Path::new(&r.path).is_file(),
                path: r.path,
                name: r.name,
                opened_at: r.opened_at,
            })
            .collect())
    }

    pub async fn forget_recent(&self, path: &str) -> Result<()> {
        Ok(self.user.forget_recent(path).await?)
    }

    // --- Designs --------------------------------------------------------------------

    pub fn validate(&self, design: &Design) -> Vec<DesignIssue> {
        validate(design, &self.registry)
    }

    fn remember(&self, path: &str) {
        self.session_paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(path.to_owned());
    }

    fn remembered(&self, path: &str) -> bool {
        self.session_paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(path)
    }

    async fn opened(&self, path: &Path) -> Result<OpenedDesign> {
        let design = kbarch::read(path, &self.registry)?;
        let p = path_string(path);
        self.user.touch_recent(&p, &design.name).await?;
        self.remember(&p);
        Ok(OpenedDesign { path: p, design })
    }

    /// Open a path the user picked in the Open dialog.
    pub async fn open_picked(&self, path: &Path) -> Result<OpenedDesign> {
        self.opened(path).await
    }

    /// Open a path from the recent-files list (the only paths the UI may name itself).
    pub async fn open_recent(&self, path: &str) -> Result<OpenedDesign> {
        if !self.user.is_recent(path).await? {
            return Err(AppError::new(
                AppErrorCode::PathNotAllowed,
                "not in the recent files",
            ));
        }
        self.opened(Path::new(path)).await
    }

    /// Save to a path the user picked in the Save As dialog. Returns the path written
    /// (with the `.kbarch` extension added if needed).
    pub async fn save_picked(&self, path: PathBuf, design: &Design) -> Result<String> {
        let path = kbarch::with_extension(path);
        self.saved(&path, design).await
    }

    /// Write an image export (M7.3) the UI rendered, to a path the user picked in the Save
    /// dialog. Exports over [`MAX_EXPORT_BYTES`] are refused (review A7).
    pub fn write_export(
        &self,
        path: PathBuf,
        format: ExportFormat,
        bytes: &[u8],
    ) -> Result<String> {
        if bytes.len() > MAX_EXPORT_BYTES {
            return Err(AppError::new(
                AppErrorCode::FileTooLarge,
                format!("an export of {} bytes", bytes.len()),
            ));
        }
        let ext = format.extension();
        let path = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
        {
            path
        } else {
            path.with_extension(ext)
        };
        std::fs::write(&path, bytes).map_err(|e| AppError::new(AppErrorCode::Io, e.to_string()))?;
        Ok(path_string(&path))
    }

    /// Import a docker-compose file the user picked in the Open dialog (M7b.1) as a new,
    /// unsaved design named after the file.
    pub fn import_compose(&self, path: &std::path::Path) -> Result<compose::ComposeImport> {
        use std::io::Read;
        let file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.take(compose::MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let text = String::from_utf8(bytes).map_err(|_| {
            AppError::new(AppErrorCode::ComposeInvalid, "the file isn't UTF-8 text")
        })?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "docker-compose".into());
        compose::import(text, &name, &self.registry)
    }

    /// Export a design as a Mermaid diagram to a path the user picked in the Save dialog
    /// (M6.6). `names` are the kind, protocol and group names in the user's language.
    pub fn export_mermaid(
        &self,
        path: PathBuf,
        design: &Design,
        names: &ka_core::mermaid::Names,
    ) -> Result<String> {
        let path = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mmd"))
        {
            path
        } else {
            path.with_extension("mmd")
        };
        std::fs::write(&path, ka_core::mermaid::to_mermaid(design, names))
            .map_err(|e| AppError::new(AppErrorCode::Io, e.to_string()))?;
        Ok(path_string(&path))
    }

    /// Save to a path opened or saved earlier this session (plain Save).
    pub async fn save_known(&self, path: &str, design: &Design) -> Result<String> {
        if !self.remembered(path) {
            return Err(AppError::new(
                AppErrorCode::PathNotAllowed,
                "save this design with Save As first",
            ));
        }
        self.saved(Path::new(path), design).await
    }

    async fn saved(&self, path: &Path, design: &Design) -> Result<String> {
        kbarch::write(path, design, &self.registry)?;
        let p = path_string(path);
        self.user.touch_recent(&p, &design.name).await?;
        self.remember(&p);
        Ok(p)
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh, empty temp folder for one test.
    pub fn temp_dir(name: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ka-app-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kbarch::tests::sample;
    use crate::testutil::temp_dir;

    async fn app(name: &str) -> (PathBuf, App) {
        let dir = temp_dir(name);
        let app = App::open(AppPaths {
            user_db: dir.join("user.db"),
            dev_data: true,
            content_db: None,
            packs_dir: None,
        })
        .await
        .unwrap();
        (dir, app)
    }

    #[tokio::test]
    async fn settings_are_allow_listed() {
        let (_dir, app) = app("settings").await;
        app.set_setting("theme", &Value::from("dark"))
            .await
            .unwrap();
        assert_eq!(
            app.get_setting("theme").await.unwrap(),
            Some(Value::from("dark"))
        );
        for key in ["window.bounds", "anything", "theme.extra"] {
            let e = app.set_setting(key, &Value::from(1)).await.unwrap_err();
            assert_eq!(e.code, AppErrorCode::SettingNotAllowed, "{key}");
            let e = app.get_setting(key).await.unwrap_err();
            assert_eq!(e.code, AppErrorCode::SettingNotAllowed, "{key}");
        }
        let big = Value::from("x".repeat(settings::MAX_SETTING_BYTES));
        assert_eq!(
            app.set_setting("sandbox.layout", &big)
                .await
                .unwrap_err()
                .code,
            AppErrorCode::InvalidInput
        );
        // Rust itself still reads and writes its own keys.
        app.set_internal_setting("window.bounds", &Value::from(1))
            .await
            .unwrap();
        assert_eq!(
            app.get_internal_setting("window.bounds").await.unwrap(),
            Some(Value::from(1))
        );
        app.close().await;
    }

    #[tokio::test]
    async fn paths_the_ui_names_must_come_from_a_dialog_or_recents() {
        let (dir, app) = app("paths").await;
        let picked = dir.join("shortener");
        let elsewhere = path_string(&dir.join("other.kbarch"));
        let d = sample();

        // Saving to a path never picked is refused; Save As adds the extension.
        assert_eq!(
            app.save_known(&elsewhere, &d).await.unwrap_err().code,
            AppErrorCode::PathNotAllowed
        );
        let saved = app.save_picked(picked, &d).await.unwrap();
        assert!(saved.ends_with("shortener.kbarch"));
        // Now plain Save works on that path.
        app.save_known(&saved, &d).await.unwrap();

        // Opening by path only works from Recents.
        std::fs::copy(&saved, &elsewhere).unwrap();
        assert_eq!(
            app.open_recent(&elsewhere).await.unwrap_err().code,
            AppErrorCode::PathNotAllowed
        );
        let opened = app.open_recent(&saved).await.unwrap();
        assert_eq!(opened.design, d);

        let recents = app.recent_files().await.unwrap();
        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0].name, d.name);
        assert!(recents[0].exists);
        std::fs::remove_file(&saved).unwrap();
        assert!(!app.recent_files().await.unwrap()[0].exists);
        assert_eq!(
            app.open_recent(&saved).await.unwrap_err().code,
            AppErrorCode::NotFound
        );
        app.forget_recent(&saved).await.unwrap();
        assert!(app.recent_files().await.unwrap().is_empty());
        app.close().await;
    }

    #[tokio::test]
    async fn a_picked_file_is_opened_and_remembered() {
        let (dir, app) = app("picked").await;
        let path = dir.join("d.kbarch");
        kbarch::write(&path, &sample(), app.registry()).unwrap();
        let opened = app.open_picked(&path).await.unwrap();
        assert_eq!(opened.design, sample());
        // It is now both in Recents and savable without a dialog.
        app.save_known(&opened.path, &opened.design).await.unwrap();
        assert_eq!(app.recent_files().await.unwrap().len(), 1);
        app.close().await;
    }

    #[tokio::test]
    async fn exports_get_their_extension_and_a_size_cap() {
        let dir = temp_dir("exports");
        let app = App::open(AppPaths {
            user_db: dir.join("user.db"),
            dev_data: false,
            content_db: None,
            packs_dir: None,
        })
        .await
        .unwrap();
        let written = app
            .write_export(dir.join("diagram"), ExportFormat::Svg, b"<svg/>")
            .unwrap();
        assert!(written.ends_with("diagram.svg"), "{written}");
        assert_eq!(std::fs::read(&written).unwrap(), b"<svg/>");
        let png = app
            .write_export(dir.join("shot.PNG"), ExportFormat::Png, &[137, 80])
            .unwrap();
        assert!(png.ends_with("shot.PNG"), "{png}");
        let huge = vec![0u8; MAX_EXPORT_BYTES + 1];
        let e = app
            .write_export(dir.join("big"), ExportFormat::Png, &huge)
            .unwrap_err();
        assert_eq!(e.code, AppErrorCode::FileTooLarge);
        app.close().await;
    }
}
