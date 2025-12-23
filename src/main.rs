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
        
        // Get path from UI property in the main thread if possible, or just used stored one
        // Since we are inside the callback, we can access UI
        let download_path = ui_handle_clone.unwrap().get_download_path().to_string();

        thread::spawn(move || {
             let ui_handle_start = ui_handle_clone.clone();
             slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle_start.upgrade() {
                    ui.set_is_downloading(true);
                    ui.set_status_text(SharedString::from("Starting download..."));
                    let mut logs = ui.get_full_log().to_string();
                    logs.push_str("--- Starting Download ---\n");
                    ui.set_full_log(SharedString::from(logs));
                }
            }).unwrap();

            let ui_handle_output = ui_handle_clone.clone();
            let result = downloader::start_download(url, format, quality, download_path, move |line| {
                let ui_handle_logging = ui_handle_output.clone();
                // Simple parsing for progress
                let progress = if line.contains("[download]") && line.contains("%") {
                     // Attempt to extract percentage, e.g. "[download]  23.5% of"
                     line.split_whitespace()
                         .find(|s| s.contains('%'))
                         .and_then(|s| s.trim_matches('%').parse::<f32>().ok())
                         .map(|p| p / 100.0)
                } else {
                    None
                };

                slint::invoke_from_event_loop(move || {
                   if let Some(ui) = ui_handle_logging.upgrade() {
                       if let Some(p) = progress {
                           ui.set_download_progress(p);
                           ui.set_status_text(SharedString::from(format!("Downloading... {:.1}%", p * 100.0)));
                       }
                       
                       // Append to full log
                       // We need to be careful not to make the string too huge, maybe truncate?
                       // For now, let's just append.
                       let mut logs = ui.get_full_log().to_string();
                       logs.push_str(&line);
                       logs.push('\n');
                       ui.set_full_log(SharedString::from(logs));
                   }
               }).unwrap();
            });

             let ui_handle_end = ui_handle_clone.clone();
             slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_handle_end.upgrade() {
                    if let Err(e) = result {
                         ui.set_status_text(SharedString::from("Error see logs"));
                         let mut logs = ui.get_full_log().to_string();
                         logs.push_str(&format!("\nERROR: {}\n", e));
                         ui.set_full_log(SharedString::from(logs));
                    } else {
                         ui.set_status_text(SharedString::from("Download finished!")); 
                         let mut logs = ui.get_full_log().to_string();
                         logs.push_str("\n--- Download Finished ---\n");
                         ui.set_full_log(SharedString::from(logs));
                    }
                     ui.set_is_downloading(false); 
                }
            }).unwrap();
        });
    });
    
    // Path Chooser
    let ui_handle_clone = ui_handle.clone();
    ui.on_choose_path(move || {
        let ui_handle = ui_handle_clone.clone();
        // rfd needs to run on main thread or be handled carefully? 
        // Sync dialog blocks, which is okay for this button on desktop usually, but might freeze UI repaint.
        // Better to spawn? rfd native dialogs block the thread calling them.
        // Let's spawn a thread to not block UI rendering loop completely, although on Windows standard file dialogs modal block app anyway.
        thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                 let path_str = path.to_string_lossy().to_string();
                 slint::invoke_from_event_loop(move || {
                     if let Some(ui) = ui_handle.upgrade() {
                         ui.set_download_path(SharedString::from(path_str));
                     }
                 }).unwrap();
            }
        });
    });

    ui.run()
}
