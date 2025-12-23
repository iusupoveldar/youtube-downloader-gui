slint::include_modules!();
use slint::SharedString;
use std::thread;
use tokio::runtime::Runtime;

mod bootstrap;
mod downloader;

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    // Create a runtime for async tasks
    let rt = Runtime::new().unwrap();

    // Bootstrap Task
    let ui_handle_clone = ui_handle.clone();
    thread::spawn(move || {
        // We need to move ui_handle_clone into this thread, and then clone it for the async block
        let ui_handle_for_async = ui_handle_clone.clone(); 
        rt.block_on(async move {
            let ui_handle_for_closure = ui_handle_for_async.clone();
            let result = bootstrap::ensure_dependencies(move |status_msg, progress| {
                let ui_handle_inner = ui_handle_for_closure.clone();
                slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle_inner.upgrade() {
                        ui.set_status_text(SharedString::from(status_msg));
                        ui.set_download_progress(progress);
                    }
                }).unwrap();
            }).await;

            let ui_handle_final = ui_handle_for_async.clone();
            if let Err(e) = result {
                 slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle_final.upgrade() {
                        ui.set_status_text(SharedString::from(format!("Error: {}", e)));
                    }
                }).unwrap();
            } else {
                 slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle_final.upgrade() {
                         ui.set_download_progress(0.0); // Reset
                         ui.set_status_text(SharedString::from("Ready to download."));
                    }
                }).unwrap();
            }
        });
    });

    // Download Handler
    let ui_handle_clone = ui_handle.clone();
    ui.on_start_download(move |url, format, quality| {
        let ui_handle_clone = ui_handle_clone.clone();
        let url = url.to_string();
        let format = format.to_string();
        let quality = quality.to_string();

        thread::spawn(move || {
             let ui_handle_start = ui_handle_clone.clone();
             slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle_start.upgrade() {
                    ui.set_is_downloading(true);
                    ui.set_status_text(SharedString::from("Starting download..."));
                }
            }).unwrap();

            let ui_handle_output = ui_handle_clone.clone();
            let result = downloader::start_download(url, format, quality, move |line| {
                let ui_handle_logging = ui_handle_output.clone();
                slint::invoke_from_event_loop(move || {
                   if let Some(ui) = ui_handle_logging.upgrade() {
                       // Try to parse percentage from line if possible, otherwise just show text
                       // For now just show last log line
                       if line.contains("[download]") {
                           ui.set_status_text(SharedString::from(line));
                       }
                   }
               }).unwrap();
            });

             let ui_handle_end = ui_handle_clone.clone();
             slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle_end.upgrade() {
                    if let Err(e) = result {
                         ui.set_status_text(SharedString::from(format!("Error: {}", e)));
                    } else {
                         ui.set_status_text(SharedString::from("Download started!")); 
                    }
                     // Quick fix: Set is_downloading false for now so user can try again
                     ui.set_is_downloading(false); 
                }
            }).unwrap();
        });
    });

    ui.run()
}
