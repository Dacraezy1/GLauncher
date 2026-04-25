use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, ScrolledWindow, ProgressBar, TextView, TextBuffer};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use crate::minecraft::launcher::{GameLauncher, LaunchEvent};
use libadwaita::ApplicationWindow;

pub fn launch_instance(
    instance_id: &str,
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) {
    // Validate account
    let account = {
        let st = state.borrow();
        let accounts = st.accounts.lock().unwrap();
        accounts.active_account().cloned()
    };

    if account.is_none() {
        let dialog = libadwaita::MessageDialog::new(
            Some(window),
            Some("No Account"),
            Some("Please add a Microsoft or Offline account before launching."),
        );
        dialog.add_response("ok", "OK");
        dialog.present();
        return;
    }

    let account = account.unwrap();

    let instance = {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(instance_id).cloned()
    };

    let Some(instance) = instance else { return; };

    // Build launch dialog
    let dialog = libadwaita::Window::new();
    dialog.set_title(Some(&format!("Launching — {}", instance.name)));
    dialog.set_transient_for(Some(window));
    dialog.set_modal(true);
    dialog.set_default_size(520, 400);
    dialog.set_resizable(false);
    dialog.set_deletable(false);

    let content = Box::new(Orientation::Vertical, 0);

    let header = libadwaita::HeaderBar::new();
    header.set_show_end_title_buttons(false);

    let close_btn = Button::with_label("Close");
    close_btn.add_css_class("flat");
    close_btn.set_sensitive(false);
    header.pack_end(&close_btn);

    content.append(&header);

    let inner = Box::new(Orientation::Vertical, 12);
    inner.set_margin_start(16);
    inner.set_margin_end(16);
    inner.set_margin_top(16);
    inner.set_margin_bottom(16);

    let status_lbl = Label::new(Some("Preparing launch..."));
    status_lbl.add_css_class("title-4");
    status_lbl.set_halign(gtk4::Align::Start);
    inner.append(&status_lbl);

    let progress = ProgressBar::new();
    progress.set_fraction(0.0);
    progress.set_pulse_step(0.1);
    inner.append(&progress);

    let log_scroll = ScrolledWindow::new();
    log_scroll.set_vexpand(true);
    log_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let log_view = TextView::new();
    log_view.set_editable(false);
    log_view.set_monospace(true);
    log_view.set_wrap_mode(gtk4::WrapMode::Word);
    log_view.add_css_class("log-view");

    let log_buffer = log_view.buffer();
    log_scroll.set_child(Some(&log_view));
    inner.append(&log_scroll);

    content.append(&inner);
    dialog.set_content(Some(&content));

    // Wire close button
    let dialog_c = dialog.clone();
    close_btn.connect_clicked(move |_| dialog_c.close());

    dialog.present();

    // Launch in background thread
    let http_client = state.borrow().http_client.clone();
    let (tx, rx) = glib::MainContext::channel::<LaunchEvent>(glib::Priority::DEFAULT);

    let instance_clone = instance.clone();
    let account_clone = account.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let launcher = GameLauncher::new(http_client.clone());

            // Find java
            let java_mgr = crate::java::JavaManager::new(http_client);
            let java_installs = java_mgr.detect_system_java();
            let required_major = instance_clone.jvm_settings.java_major_version;
            let java = java_mgr.pick_java_for_mc(required_major, &java_installs);

            let Some(java) = java else {
                let _ = tx.send(LaunchEvent::Error(
                    "No suitable Java installation found. Please download Java in the Java tab.".to_string()
                ));
                return;
            };

            let result = launcher.install_and_launch(
                &instance_clone,
                &account_clone,
                java,
                tx.clone(),
            ).await;

            if let Err(e) = result {
                let _ = tx.send(LaunchEvent::Error(e.to_string()));
            }
        });
    });

    // Handle events on main thread
    let status_lbl_clone = status_lbl.clone();
    let progress_clone = progress.clone();
    let log_buffer_clone = log_buffer.clone();
    let log_view_clone = log_view.clone();
    let close_btn_clone = close_btn.clone();
    let dialog_state = state.clone();
    let inst_id_for_time = instance.id.clone();

    let launch_start = std::time::Instant::now();

    rx.attach(None, move |event| {
        match event {
            LaunchEvent::Progress { message, percent } => {
                status_lbl_clone.set_text(&message);
                progress_clone.set_fraction(percent);
                append_log(&log_buffer_clone, &format!("[INFO] {message}"));
            }
            LaunchEvent::Log(line) => {
                append_log(&log_buffer_clone, &line);
                // Auto-scroll
                let end = log_buffer_clone.end_iter();
                log_view_clone.scroll_to_iter(&mut log_buffer_clone.end_iter(), 0.0, false, 0.0, 1.0);
            }
            LaunchEvent::Started(pid) => {
                status_lbl_clone.set_text(&format!("Game running (PID {pid})"));
                progress_clone.set_fraction(1.0);
                append_log(&log_buffer_clone, &format!("[INFO] Game started with PID {pid}"));
            }
            LaunchEvent::Exited(code) => {
                let elapsed = launch_start.elapsed().as_secs();
                let msg = if code == 0 {
                    format!("Game exited normally after {elapsed}s")
                } else {
                    format!("Game exited with code {code} after {elapsed}s")
                };
                status_lbl_clone.set_text(&msg);
                append_log(&log_buffer_clone, &format!("[INFO] {msg}"));
                close_btn_clone.set_sensitive(true);

                // Update play time
                let mut st = dialog_state.borrow_mut();
                let mut instances = st.instances.lock().unwrap();
                if let Some(inst) = instances.get_mut(&inst_id_for_time) {
                    inst.play_time_seconds += elapsed;
                    inst.last_played = Some(chrono::Utc::now().to_rfc3339());
                    let _ = inst.save();
                }

                return glib::ControlFlow::Break;
            }
            LaunchEvent::Error(e) => {
                status_lbl_clone.set_text(&format!("Error: {e}"));
                append_log(&log_buffer_clone, &format!("[ERROR] {e}"));
                progress_clone.set_fraction(0.0);
                close_btn_clone.set_sensitive(true);
                return glib::ControlFlow::Break;
            }
        }
        glib::ControlFlow::Continue
    });
}

fn append_log(buffer: &TextBuffer, line: &str) {
    let mut end = buffer.end_iter();
    buffer.insert(&mut end, &format!("{line}\n"));
}
