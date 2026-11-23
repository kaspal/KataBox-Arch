//! Problem packs (M6.4, ADR 0011): preview, apply, cancel, list and uninstall.
//!
//! The checks that open a pack (T1–T9: the archive rules, the manifest, the content and
//! a judged run of every design) live in `ka-pack`, which depends on this crate; it plugs
//! them in as a [`PackOpener`]. This module owns the rest of the threat model:
//! - T10 versions: an installed pack is replaced only by a newer version; the same version
//!   with the same SHA-256 is a no-op, with a different one it is refused; older versions
//!   are refused.
//! - T11 where files go: a pack is extracted into a fresh staging folder the app names
//!   (`packs/.staging-<n>`), moved to `packs/<id>` on Apply and deleted on Cancel or
//!   failure; every path is built from the validated pack id, never from text in the pack.
//!
//! Each installed pack keeps its own `content.db` (problems only, ids `<pack>--<id>`), so a
//! pack is live as soon as it is applied, without a restart.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ka_core::Registry;
use serde::Serialize;

use crate::{App, AppError, AppErrorCode, Result};

/// A problem a pack holds (its stored id, `<pack>--<id>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PackProblem {
    pub id: String,
    pub title_en: String,
    pub title_es: String,
    pub difficulty: String,
}

/// What [`PackOpener::open`] found in a pack that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPackData {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub licence: String,
    /// The whole `.kbpack` file's SHA-256 (hex).
    pub sha256: String,
    pub problems: Vec<PackProblem>,
}

/// Opens a `.kbpack` (implemented by `ka-pack`, plugged in by the app shell).
pub trait PackOpener: Send + Sync {
    /// Extract `archive` into `staging` (a new folder), run every check (T1–T9, the designs
    /// judged) and write `staging/content.db` with the problems as `<pack>--<id>`. On any
    /// refusal, the reasons. Blocking: the app calls it on a blocking thread.
    fn open(
        &self,
        archive: &Path,
        staging: &Path,
        builtin: &BTreeSet<String>,
        app_version: &str,
        registry: &Registry,
    ) -> std::result::Result<OpenedPackData, Vec<String>>;
}

/// `[a-z][a-z0-9]{2,19}`: no `-`, so `<pack>--<id>` never looks like a built-in id, and
/// safe as a folder name.
pub fn valid_pack_id(id: &str) -> bool {
    (3..=20).contains(&id.len())
        && id.as_bytes()[0].is_ascii_lowercase()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// `MAJOR.MINOR.PATCH` with an optional `-pre.release`: comparable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl Version {
    pub fn parse(s: &str) -> Option<Version> {
        let (core, pre) = match s.split_once('-') {
            Some((c, p))
                if !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.') =>
            {
                (c, Some(p.to_string()))
            }
            Some(_) => return None,
            None => (s, None),
        };
        let mut it = core.split('.');
        let mut num = || {
            it.next()
                .filter(|n| !n.is_empty() && n.len() <= 9 && n.bytes().all(|b| b.is_ascii_digit()))?
                .parse()
                .ok()
        };
        let v = Version {
            major: num()?,
            minor: num()?,
            patch: num()?,
            pre,
        };
        it.next().is_none().then_some(v)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(o.major, o.minor, o.patch))
            // A pre-release comes before its release.
            .then_with(|| match (&self.pre, &o.pre) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

/// What applying a previewed pack would do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackChange {
    /// Not installed before.
    New,
    /// Replaces an older installed version.
    Upgrade { from: String },
    /// Installed before and removed: comes back.
    Reinstall,
    /// The same version and contents are already installed: nothing to do.
    Same,
}

/// The import preview: shown before anything is installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PackPreview {
    /// Pass back to `apply_pack` or `cancel_pack`.
    pub token: u32,
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub licence: String,
    pub sha256: String,
    pub problems: Vec<PackProblem>,
    pub change: PackChange,
}

/// An installed (or removed) pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub licence: String,
    pub sha256: String,
    pub installed_at: i64,
    pub removed: bool,
    /// Problems in the installed pack (0 once removed).
    pub problems: u32,
}

/// A previewed pack waiting for Apply or Cancel.
pub(crate) struct Staged {
    dir: PathBuf,
    data: OpenedPackData,
    change: PackChange,
}

/// The app's pack state: where packs live, the opener, the open pack databases and the
/// previews waiting for an answer.
#[derive(Default)]
pub(crate) struct Packs {
    pub(crate) dir: Option<PathBuf>,
    pub(crate) opener: Option<Arc<dyn PackOpener>>,
    pub(crate) dbs: tokio::sync::RwLock<BTreeMap<String, ka_store::ContentDb>>,
    staged: std::sync::Mutex<HashMap<u32, Staged>>,
    next: std::sync::atomic::AtomicU32,
}

fn refused(reasons: Vec<String>) -> AppError {
    AppError::new(AppErrorCode::PackRefused, reasons.join("\n"))
}

fn io(e: std::io::Error) -> AppError {
    AppError::new(AppErrorCode::Io, e.to_string())
}

fn store(e: ka_store::StoreError) -> AppError {
    AppError::from(e)
}

impl Packs {
    pub(crate) fn new(dir: Option<PathBuf>) -> Packs {
        Packs {
            dir,
            ..Packs::default()
        }
    }

    fn root(&self) -> Result<&Path> {
        self.dir
            .as_deref()
            .ok_or_else(|| AppError::new(AppErrorCode::Internal, "no packs folder"))
    }

    /// `packs/<id>`, for a validated id only.
    fn folder(&self, id: &str) -> Result<PathBuf> {
        if !valid_pack_id(id) {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                format!("pack id {id:?}"),
            ));
        }
        Ok(self.root()?.join(id))
    }

    /// At startup: delete staging folders a crash left behind, then open every installed
    /// pack's database.
    pub(crate) async fn load(&self, installed: &[ka_store::PackRow]) -> Result<()> {
        let Some(root) = &self.dir else {
            return Ok(());
        };
        if let Ok(entries) = std::fs::read_dir(root) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with(".staging-") && e.path().is_dir() {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        let mut dbs = self.dbs.write().await;
        for p in installed.iter().filter(|p| p.removed_at.is_none()) {
            let Ok(dir) = self.folder(&p.id) else {
                continue;
            };
            let path = dir.join("content.db");
            if path.exists()
                && let Ok(db) = ka_store::ContentDb::open(&path).await
            {
                dbs.insert(p.id.clone(), db);
            }
        }
        Ok(())
    }

    pub(crate) async fn close(&self) {
        for db in self.dbs.write().await.values() {
            db.close().await;
        }
    }
}

impl App {
    /// Plug in the pack checks (from `ka-pack`). Without one, importing a pack fails.
    pub fn set_pack_opener(&mut self, opener: Arc<dyn PackOpener>) {
        self.packs.opener = Some(opener);
    }

    /// The pack database holding problem `id` (`<pack>--<problem>`), if it is installed.
    pub(crate) async fn pack_problem(&self, id: &str) -> Result<Option<ka_store::ProblemRow>> {
        let Some((pack, _)) = id.split_once("--") else {
            return Ok(None);
        };
        let dbs = self.packs.dbs.read().await;
        match dbs.get(pack) {
            Some(db) => db.problem(id).await.map_err(store),
            None => Ok(None),
        }
    }

    /// Every installed pack's problems, by pack id.
    pub(crate) async fn pack_problems(&self) -> Result<Vec<(String, ka_store::ProblemRow)>> {
        let dbs = self.packs.dbs.read().await;
        let mut out = Vec::new();
        for (id, db) in dbs.iter() {
            for row in db.problems().await.map_err(store)? {
                out.push((id.clone(), row));
            }
        }
        Ok(out)
    }

    /// Open `archive` (a path the user picked in a dialog), run every check and stage it.
    /// Nothing is installed until [`App::apply_pack`].
    pub async fn preview_pack(&self, archive: &Path) -> Result<PackPreview> {
        let opener = self
            .packs
            .opener
            .clone()
            .ok_or_else(|| AppError::new(AppErrorCode::Internal, "packs are not available"))?;
        let root = self.packs.root()?.to_path_buf();
        std::fs::create_dir_all(&root).map_err(io)?;
        let token = self
            .packs
            .next
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        // A name no earlier run used (leftovers are deleted at startup anyway).
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let staging = root.join(format!(".staging-{stamp}-{token}"));
        if staging.exists() {
            std::fs::remove_dir_all(&staging).map_err(io)?;
        }
        // A pack id may not equal a built-in problem id (or a flaw drill's).
        let mut builtin: BTreeSet<String> = match &self.content {
            Some(c) => c
                .problems()
                .await
                .map_err(store)?
                .into_iter()
                .map(|p| p.id)
                .collect(),
            None => BTreeSet::new(),
        };
        builtin.insert("flaw".into());
        let (archive, dir) = (archive.to_path_buf(), staging.clone());
        let version = self.info.version.clone();
        let registry = self.registry.clone();
        let opened = tokio::task::spawn_blocking(move || {
            opener.open(&archive, &dir, &builtin, &version, &registry)
        })
        .await
        .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
        let data = match opened {
            Ok(d) => d,
            Err(reasons) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(refused(reasons));
            }
        };
        let change = match self.pack_change(&data).await {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(e);
            }
        };
        let preview = PackPreview {
            token,
            id: data.id.clone(),
            name: data.name.clone(),
            author: data.author.clone(),
            version: data.version.clone(),
            licence: data.licence.clone(),
            sha256: data.sha256.clone(),
            problems: data.problems.clone(),
            change: change.clone(),
        };
        self.packs.staged.lock().expect("staged").insert(
            token,
            Staged {
                dir: staging,
                data,
                change,
            },
        );
        Ok(preview)
    }

    /// T10: what installing `data` would do, or why it can't.
    async fn pack_change(&self, data: &OpenedPackData) -> Result<PackChange> {
        let Some(old) = self.user.pack(&data.id).await.map_err(store)? else {
            return Ok(PackChange::New);
        };
        if old.removed_at.is_some() {
            return Ok(PackChange::Reinstall);
        }
        let version_error = |why: &str| {
            AppError::new(
                AppErrorCode::PackVersion,
                format!(
                    "{why}: {} is installed, this is {}",
                    old.version, data.version
                ),
            )
        };
        let (Some(new), Some(installed)) =
            (Version::parse(&data.version), Version::parse(&old.version))
        else {
            return Err(version_error("unreadable version"));
        };
        match new.cmp(&installed) {
            std::cmp::Ordering::Greater => Ok(PackChange::Upgrade { from: old.version }),
            std::cmp::Ordering::Equal if old.sha256.eq_ignore_ascii_case(&data.sha256) => {
                Ok(PackChange::Same)
            }
            std::cmp::Ordering::Equal => Err(version_error("same version, different contents")),
            std::cmp::Ordering::Less => Err(version_error("older version")),
        }
    }

    /// Install a previewed pack: move its staging folder to `packs/<id>` (replacing an
    /// older version), open its database and record it.
    pub async fn apply_pack(&self, token: u32) -> Result<PackInfo> {
        let staged = self
            .packs
            .staged
            .lock()
            .expect("staged")
            .remove(&token)
            .ok_or_else(|| {
                AppError::new(AppErrorCode::NotFound, format!("pack preview {token}"))
            })?;
        let d = &staged.data;
        if staged.change == PackChange::Same {
            let _ = std::fs::remove_dir_all(&staged.dir);
            return self.pack_info(&d.id).await;
        }
        let target = self.packs.folder(&d.id)?;
        let result: Result<()> = async {
            {
                let mut dbs = self.packs.dbs.write().await;
                if let Some(db) = dbs.remove(&d.id) {
                    db.close().await;
                }
            }
            if target.exists() {
                std::fs::remove_dir_all(&target).map_err(io)?;
            }
            std::fs::rename(&staged.dir, &target).map_err(io)?;
            let db = ka_store::ContentDb::open(&target.join("content.db"))
                .await
                .map_err(store)?;
            self.packs.dbs.write().await.insert(d.id.clone(), db);
            self.user
                .record_pack(&d.id, &d.name, &d.author, &d.version, &d.licence, &d.sha256)
                .await
                .map_err(store)?;
            Ok(())
        }
        .await;
        if let Err(e) = result {
            let _ = std::fs::remove_dir_all(&staged.dir);
            return Err(e);
        }
        self.pack_info(&d.id).await
    }

    /// Drop a preview and its staging folder.
    pub fn cancel_pack(&self, token: u32) {
        if let Some(s) = self.packs.staged.lock().expect("staged").remove(&token) {
            let _ = std::fs::remove_dir_all(&s.dir);
        }
    }

    async fn pack_info(&self, id: &str) -> Result<PackInfo> {
        let row = self
            .user
            .pack(id)
            .await
            .map_err(store)?
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("pack {id}")))?;
        let problems = match self.packs.dbs.read().await.get(id) {
            Some(db) => db.problems().await.map_err(store)?.len() as u32,
            None => 0,
        };
        Ok(PackInfo {
            id: row.id,
            name: row.name,
            author: row.author,
            version: row.version,
            licence: row.licence,
            sha256: row.sha256,
            installed_at: row.installed_at,
            removed: row.removed_at.is_some(),
            problems,
        })
    }

    /// Every pack ever installed (removed ones marked), by id.
    pub async fn list_packs(&self) -> Result<Vec<PackInfo>> {
        let mut out = Vec::new();
        for row in self.user.packs().await.map_err(store)? {
            out.push(self.pack_info(&row.id).await?);
        }
        Ok(out)
    }

    /// Remove an installed pack's files (`packs/<id>` only). Its row and the user's history
    /// with its problems stay.
    pub async fn uninstall_pack(&self, id: &str) -> Result<()> {
        let folder = self.packs.folder(id)?;
        match self.user.pack(id).await.map_err(store)? {
            Some(p) if p.removed_at.is_none() => {}
            _ => return Err(AppError::new(AppErrorCode::NotFound, format!("pack {id}"))),
        }
        if let Some(db) = self.packs.dbs.write().await.remove(id) {
            db.close().await;
        }
        if folder.exists() {
            std::fs::remove_dir_all(&folder).map_err(io)?;
        }
        self.user.mark_pack_removed(id).await.map_err(store)
    }

    /// The name of the pack a problem came from (`<pack>--<id>`), installed or removed.
    pub(crate) async fn pack_name_of(&self, problem_id: &str) -> Result<Option<String>> {
        match problem_id.split_once("--") {
            Some((pack, _)) if pack != "flaw" => {
                Ok(self.user.pack(pack).await.map_err(store)?.map(|p| p.name))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;

    const PROBLEM: &str = r#"
id = "demo"
number = 1
set = 1
difficulty = "easy"
title = "Demo"
title_es = "Demo"
teaches = "Scaling"
teaches_es = "Escalar"

[nfr]
peak_rps = 100
p95_ms = 100
availability = 0.999
budget_usd = 2000

[[request_types]]
id = "read"
share = 1.0
route = ["compute"]

[[tests]]
id = "normal"
name = "Normal day"
name_es = "Un dia normal"

[pass]
error_rate = 0.001
"#;

    /// Stands in for `ka-pack`: "opens" a pack with a fixed manifest, writing a one-problem
    /// content.db into the staging folder, or refuses it.
    struct Fake {
        id: &'static str,
        version: &'static str,
        sha: &'static str,
        refuse: bool,
    }

    impl PackOpener for Fake {
        fn open(
            &self,
            _archive: &Path,
            staging: &Path,
            builtin: &BTreeSet<String>,
            _app_version: &str,
            _registry: &Registry,
        ) -> std::result::Result<OpenedPackData, Vec<String>> {
            std::fs::create_dir_all(staging).unwrap();
            if self.refuse || builtin.contains(self.id) {
                return Err(vec!["problems/demo/statement.md: a <script> tag".into()]);
            }
            let mut def: ka_core::problem::ProblemDef = toml::from_str(PROBLEM).unwrap();
            let id = format!("{}--demo", self.id);
            def.id = id.clone();
            let row = ka_store::ProblemRow {
                id: id.clone(),
                number: 1,
                set: 1,
                difficulty: "easy".into(),
                title_en: "Demo".into(),
                title_es: "Demo".into(),
                teaches_en: "Scaling".into(),
                teaches_es: "Escalar".into(),
                tags: vec![],
                nfr: serde_json::to_string(&def.nfr).unwrap(),
                def: serde_json::to_string(&def).unwrap(),
                statement_en: "# Demo".into(),
                statement_es: "# Demo".into(),
                hints_en: vec!["a".into(), "b".into(), "c".into()],
                hints_es: vec!["a".into(), "b".into(), "c".into()],
                editorial_en: "E".into(),
                editorial_es: "E".into(),
                reference: String::new(),
                ref_cost: 1.0,
                ref_p95_ms: 1.0,
                track: "core".into(),
                challenge_ratio: None,
            };
            let db = staging.join("content.db");
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(ka_store::write_content(
                    &db,
                    &ka_store::ContentData {
                        problems: vec![row],
                        ..Default::default()
                    },
                ))
                .unwrap();
            Ok(OpenedPackData {
                id: self.id.into(),
                name: "Drills".into(),
                author: "Ana".into(),
                version: self.version.into(),
                licence: "MIT".into(),
                sha256: self.sha.into(),
                problems: vec![PackProblem {
                    id,
                    title_en: "Demo".into(),
                    title_es: "Demo".into(),
                    difficulty: "easy".into(),
                }],
            })
        }
    }

    async fn app(name: &str) -> (App, PathBuf) {
        let dir = temp_dir(name);
        let app = App::open(AppPaths {
            user_db: dir.join("user.db"),
            dev_data: false,
            content_db: None,
            packs_dir: Some(dir.join("packs")),
        })
        .await
        .unwrap();
        (app, dir.join("packs"))
    }

    fn opener(app: &mut App, version: &'static str, sha: &'static str) {
        app.set_pack_opener(Arc::new(Fake {
            id: "drills",
            version,
            sha,
            refuse: false,
        }));
    }

    fn staging_left(packs: &Path) -> usize {
        std::fs::read_dir(packs)
            .map(|d| {
                d.flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with(".staging-"))
                    .count()
            })
            .unwrap_or(0)
    }

    #[tokio::test]
    async fn a_pack_installs_live_and_uninstalls_without_touching_history() {
        let (mut app, packs) = app("pack-install").await;
        opener(&mut app, "1.0.0", "aa");
        let p = app.preview_pack(Path::new("x.kbpack")).await.unwrap();
        assert_eq!((p.id.as_str(), &p.change), ("drills", &PackChange::New));
        assert_eq!(staging_left(&packs), 1, "staged, not installed");
        assert!(
            app.list_problems().await.is_err() || app.pack_problems().await.unwrap().is_empty()
        );
        let info = app.apply_pack(p.token).await.unwrap();
        assert_eq!((info.problems, info.removed), (1, false));
        assert_eq!(staging_left(&packs), 0);
        assert!(packs.join("drills/content.db").exists());
        // Live without a restart: the problem opens by its stored id.
        let row = app.problem_row("drills--demo").await.unwrap();
        assert_eq!(row.title_en, "Demo");
        assert_eq!(
            app.pack_name_of("drills--demo").await.unwrap().as_deref(),
            Some("Drills")
        );
        // Uninstall removes packs/drills only; the row stays, marked removed.
        app.uninstall_pack("drills").await.unwrap();
        assert!(!packs.join("drills").exists());
        let list = app.list_packs().await.unwrap();
        assert_eq!((list.len(), list[0].removed), (1, true));
        assert!(app.problem_row("drills--demo").await.is_err());
        // Only a validated id names a folder.
        let e = app.uninstall_pack("../x").await.unwrap_err();
        assert_eq!(e.code, AppErrorCode::InvalidInput);
        // A removed pack comes back as a reinstall.
        let p = app.preview_pack(Path::new("x.kbpack")).await.unwrap();
        assert_eq!(p.change, PackChange::Reinstall);
        app.cancel_pack(p.token);
        assert_eq!(staging_left(&packs), 0, "cancel deletes the staging folder");
        app.close().await;
    }

    #[tokio::test]
    async fn t10_versions_only_move_forward() {
        let (mut app, packs) = app("pack-versions").await;
        opener(&mut app, "1.1.0", "aa");
        let p = app.preview_pack(Path::new("x.kbpack")).await.unwrap();
        app.apply_pack(p.token).await.unwrap();
        // The same version with the same contents: a no-op.
        let p = app.preview_pack(Path::new("x.kbpack")).await.unwrap();
        assert_eq!(p.change, PackChange::Same);
        assert_eq!(app.apply_pack(p.token).await.unwrap().version, "1.1.0");
        // The same version with different contents, or an older one: refused, and the
        // staging folder is gone.
        for (v, sha) in [("1.1.0", "bb"), ("1.0.9", "cc"), ("1.1.0-beta.1", "dd")] {
            opener(&mut app, v, sha);
            let e = app.preview_pack(Path::new("x.kbpack")).await.unwrap_err();
            assert_eq!(e.code, AppErrorCode::PackVersion, "{v}");
            assert_eq!(staging_left(&packs), 0, "{v}");
        }
        // A newer one replaces it.
        opener(&mut app, "1.2.0", "ee");
        let p = app.preview_pack(Path::new("x.kbpack")).await.unwrap();
        assert_eq!(
            p.change,
            PackChange::Upgrade {
                from: "1.1.0".into()
            }
        );
        assert_eq!(app.apply_pack(p.token).await.unwrap().version, "1.2.0");
        assert_eq!(
            app.problem_row("drills--demo").await.unwrap().id,
            "drills--demo"
        );
        app.close().await;
    }

    #[tokio::test]
    async fn a_refused_pack_leaves_nothing_behind() {
        let (mut app, packs) = app("pack-refused").await;
        app.set_pack_opener(Arc::new(Fake {
            id: "drills",
            version: "1.0.0",
            sha: "aa",
            refuse: true,
        }));
        let e = app.preview_pack(Path::new("x.kbpack")).await.unwrap_err();
        assert_eq!(e.code, AppErrorCode::PackRefused);
        assert!(e.message.contains("<script>"));
        assert_eq!(staging_left(&packs), 0);
        assert!(app.list_packs().await.unwrap().is_empty());
        // A leftover staging folder (a crash mid-import) is deleted at the next start.
        std::fs::create_dir_all(packs.join(".staging-1-1/problems")).unwrap();
        app.close().await;
        let (app, packs) = (
            App::open(AppPaths {
                user_db: packs.parent().unwrap().join("user.db"),
                dev_data: false,
                content_db: None,
                packs_dir: Some(packs.clone()),
            })
            .await
            .unwrap(),
            packs,
        );
        assert_eq!(staging_left(&packs), 0);
        app.close().await;
    }

    #[test]
    fn versions_compare_with_pre_releases_first() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("1.0.0") > v("1.0.0-beta.1"));
        assert!(v("1.2.0") < v("1.10.0-rc.1"));
        assert!(v("2.0.0") > v("1.99.99"));
        for bad in ["1.0", "1.0.0.0", "a.b.c", "1.0.0-", "01x.0.0", ""] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn pack_ids_are_safe_folder_names() {
        assert!(valid_pack_id("graphs101"));
        for bad in [
            "ab",
            "two-sum",
            "Graphs",
            "1abc",
            "a_b_c",
            "..",
            "a/b",
            "abcdefghijklmnopqrstu",
        ] {
            assert!(!valid_pack_id(bad), "{bad}");
        }
    }
}
