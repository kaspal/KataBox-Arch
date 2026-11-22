//! The Tauri shell: typed commands that forward to `ka_app::App`, Rust-side file
//! dialogs, the window title, the unsaved-changes close guard and window memory. No
//! business logic lives here (CLAUDE.md).
//!
//! Window config notes (tauri.conf.json can't hold comments):
//! - `visible: false`: the window is placed from the saved bounds, then shown
//!   (`window_state`).
//! - `dragDropEnabled: false`: on Windows, Tauri's window-level file-drop handler is on by
//!   default and swallows HTML5 drag events, so dragging a component from the library onto
//!   the canvas would silently do nothing. Dropping a `.kbarch` file onto the window to
//!   open it would need this revisited (backlog).

mod window_state;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use ka_app::guide::{GuidePage, GuideSummary};
use ka_app::live::FrameSink;
use ka_app::patterns::{PatternPage, PatternSummary};
use ka_app::problems::{ProblemBrief, ProblemSummary, Progress, SubmitResult};
use ka_app::samples::{Sample, SampleSummary};
use ka_app::{
    App, AppError, AppErrorCode, AppInfo, AppPaths, OpenedDesign, ProblemTest, RecentEntry,
    RunControl, RunInfo, RunRequest, SandboxPreset,
};
use ka_core::{Design, DesignIssue, Registry};
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody, JavaScriptChannelId};
use tauri::{Emitter, Manager, State, Webview, WebviewWindow, WindowEvent};

type Res<T> = Result<T, AppError>;

/// The opened app, or why it could not open (every command then returns that error,
/// and the UI shows it instead of crashing).
struct AppState(Result<App, AppError>);

impl AppState {
    fn app(&self) -> Res<&App> {
        self.0.as_ref().map_err(Clone::clone)
    }
}

/// The close guard: the UI reports unsaved changes; closing a dirty window asks first.
#[derive(Default)]
struct CloseGuard {
    dirty: AtomicBool,
}

/// The event the UI listens for when a close was held back by unsaved changes.
const CLOSE_REQUESTED_EVENT: &str = "close-requested";

const KBARCH_FILTER: &str = "KataBox Arch design";
const KBARCH_EXTENSIONS: &[&str] = &["kbarch"];

// --- App and settings -----------------------------------------------------------------

#[tauri::command]
#[specta::specta]
async fn app_info(state: State<'_, AppState>) -> Res<AppInfo> {
    Ok(state.app()?.info())
}

/// The component registry and wiring table (fetched once by the UI).
#[tauri::command]
#[specta::specta]
async fn registry(state: State<'_, AppState>) -> Res<Registry> {
    Ok(state.app()?.registry().clone())
}

#[tauri::command]
#[specta::specta]
async fn get_setting(state: State<'_, AppState>, key: String) -> Res<Option<Value>> {
    state.app()?.get_setting(&key).await
}

#[tauri::command]
#[specta::specta]
async fn set_setting(state: State<'_, AppState>, key: String, value: Value) -> Res<()> {
    state.app()?.set_setting(&key, &value).await
}

// --- Files ----------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
async fn recent_files(state: State<'_, AppState>) -> Res<Vec<RecentEntry>> {
    state.app()?.recent_files().await
}

#[tauri::command]
#[specta::specta]
async fn forget_recent(state: State<'_, AppState>, path: String) -> Res<()> {
    state.app()?.forget_recent(&path).await
}

/// Show the Open dialog and read the picked design. `None` when cancelled.
#[tauri::command]
#[specta::specta]
async fn open_design_dialog(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Res<Option<OpenedDesign>> {
    let svc = state.app()?;
    match pick_file(app, None).await? {
        Some(path) => Ok(Some(svc.open_picked(&path).await?)),
        None => Ok(None),
    }
}

/// Open a design from the recent-files list.
#[tauri::command]
#[specta::specta]
async fn open_recent(state: State<'_, AppState>, path: String) -> Res<OpenedDesign> {
    state.app()?.open_recent(&path).await
}

/// Save to a path opened or saved earlier this session. Returns the path written.
#[tauri::command]
#[specta::specta]
async fn save_design(state: State<'_, AppState>, path: String, design: Design) -> Res<String> {
    state.app()?.save_known(&path, &design).await
}

/// Show the Save As dialog (suggesting `suggested_name`) and save. `None` when cancelled.
#[tauri::command]
#[specta::specta]
async fn save_design_as(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    design: Design,
    suggested_name: String,
) -> Res<Option<String>> {
    let svc = state.app()?;
    match pick_file(app, Some(suggested_name)).await? {
        Some(path) => Ok(Some(svc.save_picked(path, &design).await?)),
        None => Ok(None),
    }
}

#[tauri::command]
#[specta::specta]
async fn validate_design(state: State<'_, AppState>, design: Design) -> Res<Vec<DesignIssue>> {
    Ok(state.app()?.validate(&design))
}

async fn pick_file(app: tauri::AppHandle, save_as: Option<String>) -> Res<Option<PathBuf>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let dialog = app
            .dialog()
            .file()
            .add_filter(KBARCH_FILTER, KBARCH_EXTENSIONS);
        match save_as {
            Some(name) => dialog.set_file_name(name).blocking_save_file(),
            None => dialog.blocking_pick_file(),
        }
    })
    .await
    .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
    picked
        .map(|p| {
            p.into_path()
                .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))
        })
        .transpose()
}

// --- Live runs ------------------------------------------------------------------------

/// Start a live Sandbox run. Frames (binary, `KAF1`) stream to the channel the UI created;
/// they arrive in JavaScript as ArrayBuffers.
#[derive(serde::Deserialize, specta::Type)]
struct StartRun {
    design: Design,
    preset: SandboxPreset,
    /// Run a problem's test instead of the Sandbox preset.
    problem: Option<ProblemTest>,
    speed: f64,
    /// Chaos the user injected (each at the moment it was injected).
    chaos: Vec<ka_core::scenario::ChaosEvent>,
    /// Where playback starts, in simulated seconds.
    start_s: f64,
    /// A `Channel` from `@tauri-apps/api/core`, serialized as its id.
    #[specta(type = String)]
    on_frame: JavaScriptChannelId,
}

/// Sends frames over a Tauri channel as raw bytes (never JSON per packet).
struct ChannelSink(Channel<InvokeResponseBody>);

impl FrameSink for ChannelSink {
    fn send(&mut self, frame: Vec<u8>) -> bool {
        self.0.send(InvokeResponseBody::Raw(frame)).is_ok()
    }
}

#[tauri::command]
#[specta::specta]
async fn start_run(
    webview: Webview,
    state: State<'_, AppState>,
    request: StartRun,
) -> Res<RunInfo> {
    let sink = ChannelSink(request.on_frame.channel_on(webview));
    let app = state.app()?;
    if let Some(p) = &request.problem {
        app.record_problem_run(&p.problem_id).await?;
    }
    app.start_run(
        &request.design,
        RunRequest {
            preset: request.preset,
            problem: request.problem,
            speed: request.speed,
            chaos: request.chaos,
            start_s: request.start_s,
        },
        sink,
    )
    .await
}

#[tauri::command]
#[specta::specta]
async fn control_run(state: State<'_, AppState>, run_id: u32, control: RunControl) -> Res<()> {
    state.app()?.control_run(run_id, control)
}

#[tauri::command]
#[specta::specta]
async fn run_result(state: State<'_, AppState>, run_id: u32) -> Res<Option<ka_sim::RunResult>> {
    state.app()?.run_result(run_id)
}

/// How much traffic the design holds before it fails, and what fails first (a few seconds of
/// simulation, off the async runtime).
#[tauri::command]
#[specta::specta]
async fn find_breaking_point(
    state: State<'_, AppState>,
    design: Design,
) -> Res<ka_sim::BreakingPoint> {
    let registry = state.app()?.registry().clone();
    tauri::async_runtime::spawn_blocking(move || ka_app::breaking_point(&design, &registry))
        .await
        .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?
}

// --- Problems -------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
async fn list_problems(state: State<'_, AppState>) -> Res<Vec<ProblemSummary>> {
    state.app()?.list_problems().await
}

/// The Component Guide index: every entry's one-liner and read mark.
#[tauri::command]
#[specta::specta]
async fn list_guide(state: State<'_, AppState>) -> Res<Vec<GuideSummary>> {
    state.app()?.list_guide().await
}

/// One Guide entry.
#[tauri::command]
#[specta::specta]
async fn guide_page(state: State<'_, AppState>, kind: ka_core::ComponentKind) -> Res<GuidePage> {
    state.app()?.guide_page(kind).await
}

/// Mark a Guide entry read (or unread).
#[tauri::command]
#[specta::specta]
async fn set_guide_read(
    state: State<'_, AppState>,
    kind: ka_core::ComponentKind,
    read: bool,
) -> Res<()> {
    state.app()?.set_guide_read(kind, read).await
}

/// The "Which one do I need?" decision guides, in order.
#[tauri::command]
#[specta::specta]
async fn decide_flows(state: State<'_, AppState>) -> Res<Vec<ka_core::decide::DecideFlow>> {
    state.app()?.decide_flows().await
}

/// The glossary.
#[tauri::command]
#[specta::specta]
async fn glossary(state: State<'_, AppState>) -> Res<Vec<ka_core::decide::GlossaryTerm>> {
    state.app()?.glossary().await
}

/// The numbers cheat sheet (M5.1).
#[tauri::command]
#[specta::specta]
async fn numbers(state: State<'_, AppState>) -> Res<Vec<ka_core::numbers::NumberFigure>> {
    state.app()?.numbers().await
}

/// Estimation drills (all, or one problem's), without their answers.
#[tauri::command]
#[specta::specta]
async fn drills(
    state: State<'_, AppState>,
    problem_id: Option<String>,
) -> Res<Vec<ka_core::estimate::Drill>> {
    state.app()?.drills(problem_id.as_deref()).await
}

/// Check an estimate: the verdict, the answer and the worked solution (recorded).
#[tauri::command]
#[specta::specta]
async fn check_estimate(
    state: State<'_, AppState>,
    problem_id: String,
    drill_id: String,
    answer: f64,
) -> Res<ka_core::estimate::EstimateCheck> {
    state
        .app()?
        .check_estimate(&problem_id, &drill_id, answer)
        .await
}

/// The estimation drills answered so far.
#[tauri::command]
#[specta::specta]
async fn estimate_statuses(state: State<'_, AppState>) -> Res<Vec<ka_store::EstimateStatus>> {
    state.app()?.estimate_statuses().await
}

/// The Find-the-flaw drills (each opens as the problem `flaw--<id>`).
#[tauri::command]
#[specta::specta]
async fn list_flaws(state: State<'_, AppState>) -> Res<Vec<ka_core::flaw::FlawSummary>> {
    state.app()?.list_flaws().await
}

/// The pattern library, in order (each with its template for the preview).
#[tauri::command]
#[specta::specta]
async fn list_patterns(state: State<'_, AppState>) -> Res<Vec<PatternSummary>> {
    state.app()?.list_patterns().await
}

/// One pattern: its page and template.
#[tauri::command]
#[specta::specta]
async fn pattern(state: State<'_, AppState>, id: String) -> Res<PatternPage> {
    state.app()?.pattern(&id).await
}

/// Mark a pattern read (or unread).
#[tauri::command]
#[specta::specta]
async fn set_pattern_read(state: State<'_, AppState>, id: String, read: bool) -> Res<()> {
    state.app()?.set_pattern_read(&id, read).await
}

/// A component kind's concept card (null until it has one).
#[tauri::command]
#[specta::specta]
async fn concept(state: State<'_, AppState>, kind: ka_core::ComponentKind) -> Res<Option<Sample>> {
    state.app()?.concept(kind).await
}

/// The sample gallery, in order (each with its first step's design for the preview).
#[tauri::command]
#[specta::specta]
async fn list_samples(state: State<'_, AppState>) -> Res<Vec<SampleSummary>> {
    state.app()?.list_samples().await
}

/// One sample for the walkthrough player: its steps and design snapshots.
#[tauri::command]
#[specta::specta]
async fn sample(state: State<'_, AppState>, id: String) -> Res<Sample> {
    state.app()?.sample(&id).await
}

/// A problem's brief: hidden tests by name only, hints up to the unlocked tier.
#[tauri::command]
#[specta::specta]
async fn problem_brief(state: State<'_, AppState>, id: String) -> Res<ProblemBrief> {
    state.app()?.problem_brief(&id).await
}

#[tauri::command]
#[specta::specta]
async fn unlock_hint(state: State<'_, AppState>, id: String, tier: i64) -> Res<()> {
    state.app()?.unlock_hint(&id, tier).await
}

#[tauri::command]
#[specta::specta]
async fn save_draft(state: State<'_, AppState>, id: String, design: Design) -> Res<()> {
    state.app()?.save_draft(&id, &design).await
}

/// Judge the design on every test of the problem (off the async runtime), then record it.
/// `local_day` is the user's date (`YYYY-MM-DD`), for the streak.
#[tauri::command]
#[specta::specta]
async fn submit_design(
    state: State<'_, AppState>,
    id: String,
    design: Design,
    local_day: String,
) -> Res<SubmitResult> {
    let app = state.app()?;
    let (problem, ref_cost) = app.problem_for_judging(&id).await?;
    let registry = app.registry().clone();
    let judged = design.clone();
    let judgement = tauri::async_runtime::spawn_blocking(move || {
        ka_app::problems::judge_design(&problem, &judged, &registry, ref_cost)
    })
    .await
    .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
    app.record_submission(&design, judgement, &local_day).await
}

#[tauri::command]
#[specta::specta]
async fn submissions(
    state: State<'_, AppState>,
    id: String,
) -> Res<Vec<ka_store::SubmissionSummary>> {
    state.app()?.submissions(&id).await
}

/// Solved and attempted per difficulty, the streak (from the user's `today`) and what next.
#[tauri::command]
#[specta::specta]
async fn progress(state: State<'_, AppState>, today: String) -> Res<Progress> {
    state.app()?.progress(&today).await
}

// --- Window ---------------------------------------------------------------------------

/// The window title (the webview has no `setTitle` permission).
#[tauri::command]
#[specta::specta]
async fn set_window_title(window: WebviewWindow, title: String) -> Res<()> {
    let title: String = title.chars().take(200).collect();
    window
        .set_title(&title)
        .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))
}

/// The UI reports whether the open design has unsaved changes.
#[tauri::command]
#[specta::specta]
async fn set_dirty(guard: State<'_, CloseGuard>, dirty: bool) -> Res<()> {
    guard.dirty.store(dirty, Ordering::SeqCst);
    Ok(())
}

/// Close the window after the user chose Save or Discard in the unsaved-changes dialog.
#[tauri::command]
#[specta::specta]
async fn confirm_close(window: WebviewWindow, guard: State<'_, CloseGuard>) -> Res<()> {
    guard.dirty.store(false, Ordering::SeqCst);
    window_state::save_now(&window);
    window
        .destroy()
        .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))
}

/// Every close (title-bar X, Alt+F4, the taskbar's Close) arrives as `CloseRequested`:
/// with unsaved changes it is held back and the UI asks Save / Discard / Cancel.
fn guard_close(window: &WebviewWindow) {
    let w = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event
            && w.try_state::<CloseGuard>()
                .is_some_and(|g| g.dirty.load(Ordering::SeqCst))
        {
            api.prevent_close();
            let _ = w.emit_to(w.label(), CLOSE_REQUESTED_EVENT, ());
        }
    });
}

// --- Setup ----------------------------------------------------------------------------

fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        app_info,
        registry,
        get_setting,
        set_setting,
        recent_files,
        forget_recent,
        open_design_dialog,
        open_recent,
        save_design,
        save_design_as,
        validate_design,
        start_run,
        control_run,
        run_result,
        find_breaking_point,
        list_problems,
        problem_brief,
        unlock_hint,
        save_draft,
        submit_design,
        submissions,
        progress,
        list_samples,
        sample,
        decide_flows,
        glossary,
        numbers,
        drills,
        check_estimate,
        estimate_statuses,
        list_flaws,
        list_patterns,
        pattern,
        set_pattern_read,
        list_guide,
        concept,
        guide_page,
        set_guide_read,
        set_window_title,
        set_dirty,
        confirm_close,
    ])
}

const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");
const REGISTRY_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../src/test/fixtures/registry.json"
);

/// Write `apps/desktop/src/bindings.ts` (the export-bindings bin; debug builds of the app
/// also do it at startup). The file is checked in and `pnpm build` typechecks it.
pub fn export_bindings() {
    export_bindings_from(&specta_builder());
}

fn export_bindings_from(builder: &tauri_specta::Builder<tauri::Wry>) {
    let ts = specta_typescript::Typescript::default()
        .header(
            "// Generated by tauri-specta from apps/desktop/src-tauri/src/lib.rs. Do not edit.\n\
             // Regenerate: cargo run -p katabox-arch-desktop --features bindings --bin export-bindings\n\
             // @ts-nocheck",
        )
        // Times (ms) are 64-bit in Rust but far below 2^53.
        .bigint(specta_typescript::BigIntExportBehavior::Number);
    builder
        .export(ts, BINDINGS)
        .expect("failed to export TypeScript bindings");
}

/// Write the registry (as the `registry` command returns it) plus Rust's kind × kind
/// wiring matrix, for UI tests only: the fake API serves it, and the wiring parity test
/// checks the TypeScript evaluator reproduces the matrix. App code never imports it.
pub fn export_registry_fixture() {
    let registry = ka_core::registry();
    let matrix = registry.wiring.matrix();
    let fixture = serde_json::json!({
        "registry": registry,
        "matrix": matrix,
        "schema_version": ka_core::SCHEMA_VERSION,
    });
    let mut text = serde_json::to_string_pretty(&fixture).expect("serialize the registry fixture");
    text.push('\n');
    let path = std::path::Path::new(REGISTRY_FIXTURE);
    std::fs::create_dir_all(path.parent().expect("fixtures folder")).expect("create fixtures/");
    std::fs::write(path, text).expect("write the registry fixture");
}

const FRAME_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../src/test/fixtures/frame.kaf"
);

/// Write a live-run frame Rust encoded, for the UI decoder's test only.
pub fn export_frame_fixture() {
    std::fs::write(FRAME_FIXTURE, ka_app::live::fixture_frame()).expect("write the frame fixture");
}

/// Debug builds keep their data in `<identifier>-dev` folders, so development and testing
/// never touch the user's real user.db.
fn data_folder(dir: PathBuf) -> PathBuf {
    if cfg!(debug_assertions) {
        let mut name = dir.file_name().unwrap_or_default().to_os_string();
        name.push("-dev");
        dir.with_file_name(name)
    } else {
        dir
    }
}

fn open_app(handle: &tauri::AppHandle) -> Res<App> {
    let dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::new(AppErrorCode::Internal, format!("app data folder: {e}")))?;
    // content.db is a bundled resource (built from content/ by build.rs).
    let content_db = handle
        .path()
        .resolve("content.db", tauri::path::BaseDirectory::Resource)
        .ok();
    let paths = AppPaths {
        user_db: data_folder(dir).join("user.db"),
        dev_data: cfg!(debug_assertions),
        content_db,
    };
    tauri::async_runtime::block_on(App::open(paths))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_app() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    {
        export_bindings_from(&builder);
        export_registry_fixture();
        export_frame_fixture();
    }

    let app = tauri::Builder::default()
        // File pickers, used only from Rust commands: the webview gets no dialog permission.
        .plugin(tauri_plugin_dialog::init());
    // Auto-update is prepared but off: only a build with `--features updater` (and
    // tauri.updater.conf.json's key and endpoint) registers the plugin.
    #[cfg(feature = "updater")]
    let app = app.plugin(tauri_plugin_updater::Builder::new().build());
    app.invoke_handler(builder.invoke_handler())
        .setup(|app| {
            let state = open_app(app.handle());
            if let Err(e) = &state {
                eprintln!("KataBox Arch could not start: {e}");
            }
            app.manage(AppState(state));
            app.manage(CloseGuard::default());
            // The main window starts hidden: place it where it was, then show it.
            app.manage(window_state::Saved::default());
            if let Some(window) = app.get_webview_window("main") {
                window_state::restore(&window);
                window_state::watch(&window);
                guard_close(&window);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building KataBox Arch")
        .run(|_, _| {});
}
