//! Print the page the main webview shows to a PDF file (M7b.3, review A8).
//!
//! Windows: WebView2's `ICoreWebView2_7::PrintToPdf`, called from Rust on the webview's own
//! thread (`with_webview`), with its completion handler answering over a channel. The webview
//! gets no new permission and the CSP doesn't change: the page only asks for the print, by
//! token, once it says it is ready.
//!
//! Linux (WebKitGTK) isn't wired yet: `print` returns `PdfUnsupported`, and the UI offers the
//! print-ready HTML and the system print dialog instead (`docs/linux-webkitgtk.md`).

use std::path::PathBuf;
use std::time::Duration;

use ka_app::{AppError, AppErrorCode};
use tauri::WebviewWindow;

/// How long a print may take before it counts as failed.
pub const PDF_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(windows)]
pub async fn print(window: &WebviewWindow, path: PathBuf) -> Result<(), AppError> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, ICoreWebView2_7, ICoreWebView2Environment6,
    };
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::core::{HSTRING, Interface};

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), AppError>>();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
    let answer = {
        let tx = tx.clone();
        move |r: Result<(), AppError>| {
            if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
                let _ = tx.send(r);
            }
        }
    };
    let failed = |what: &str, e: windows::core::Error| {
        AppError::new(AppErrorCode::PdfFailed, format!("{what}: {e}"))
    };
    window
        .with_webview(move |wv| {
            let start = || -> Result<(), AppError> {
                // SAFETY: plain COM calls on the webview's own thread, which `with_webview`
                // runs this closure on; every interface comes from the live webview.
                unsafe {
                    let core = wv
                        .controller()
                        .CoreWebView2()
                        .map_err(|e| failed("the webview", e))?;
                    let core7: ICoreWebView2_7 = core.cast().map_err(|_| {
                        AppError::new(
                            AppErrorCode::PdfUnsupported,
                            "this WebView2 runtime can't print to PDF",
                        )
                    })?;
                    let env6: ICoreWebView2Environment6 =
                        wv.environment().cast().map_err(|_| {
                            AppError::new(
                                AppErrorCode::PdfUnsupported,
                                "this WebView2 runtime has no print settings",
                            )
                        })?;
                    let settings = env6
                        .CreatePrintSettings()
                        .map_err(|e| failed("print settings", e))?;
                    settings
                        .SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)
                        .map_err(|e| failed("orientation", e))?;
                    // 15 mm margins, in inches; no browser header, footer or backgrounds.
                    let m = 15.0 / 25.4;
                    for set in [
                        ICoreWebView2PrintSettingsExt::SetMarginTop,
                        ICoreWebView2PrintSettingsExt::SetMarginBottom,
                        ICoreWebView2PrintSettingsExt::SetMarginLeft,
                        ICoreWebView2PrintSettingsExt::SetMarginRight,
                    ] {
                        set(&settings, m).map_err(|e| failed("margins", e))?;
                    }
                    settings
                        .SetShouldPrintBackgrounds(false)
                        .map_err(|e| failed("backgrounds", e))?;
                    settings
                        .SetShouldPrintHeaderAndFooter(false)
                        .map_err(|e| failed("header and footer", e))?;
                    let done = answer.clone();
                    let handler = PrintToPdfCompletedHandler::create(Box::new(move |hr, ok| {
                        done(match hr {
                            Ok(()) if ok => Ok(()),
                            Ok(()) => Err(AppError::new(
                                AppErrorCode::PdfFailed,
                                "the web view reported the PDF wasn't written",
                            )),
                            Err(e) => Err(AppError::new(AppErrorCode::PdfFailed, e.to_string())),
                        });
                        Ok(())
                    }));
                    core7
                        .PrintToPdf(&HSTRING::from(path.as_os_str()), &settings, &handler)
                        .map_err(|e| failed("print", e))?;
                }
                Ok(())
            };
            if let Err(e) = start() {
                answer(Err(e));
            }
        })
        .map_err(|e| AppError::new(AppErrorCode::PdfFailed, e.to_string()))?;
    match tokio::time::timeout(PDF_TIMEOUT, rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => Err(AppError::new(
            AppErrorCode::PdfFailed,
            "the print was dropped",
        )),
        Err(_) => Err(AppError::new(
            AppErrorCode::PdfTimeout,
            "printing to PDF took more than a minute",
        )),
    }
}

/// The margin setters, named so they can be looped over.
#[cfg(windows)]
struct ICoreWebView2PrintSettingsExt;

#[cfg(windows)]
impl ICoreWebView2PrintSettingsExt {
    #[allow(non_snake_case)]
    fn SetMarginTop(
        s: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings,
        v: f64,
    ) -> windows::core::Result<()> {
        // SAFETY: a COM setter on a live settings object.
        unsafe { s.SetMarginTop(v) }
    }
    #[allow(non_snake_case)]
    fn SetMarginBottom(
        s: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings,
        v: f64,
    ) -> windows::core::Result<()> {
        // SAFETY: as above.
        unsafe { s.SetMarginBottom(v) }
    }
    #[allow(non_snake_case)]
    fn SetMarginLeft(
        s: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings,
        v: f64,
    ) -> windows::core::Result<()> {
        // SAFETY: as above.
        unsafe { s.SetMarginLeft(v) }
    }
    #[allow(non_snake_case)]
    fn SetMarginRight(
        s: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings,
        v: f64,
    ) -> windows::core::Result<()> {
        // SAFETY: as above.
        unsafe { s.SetMarginRight(v) }
    }
}

#[cfg(not(windows))]
pub async fn print(_window: &WebviewWindow, _path: PathBuf) -> Result<(), AppError> {
    Err(AppError::new(
        AppErrorCode::PdfUnsupported,
        "printing to PDF isn't available on this system yet",
    ))
}
