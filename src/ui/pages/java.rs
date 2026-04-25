use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, ScrolledWindow, Image};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use libadwaita::ApplicationWindow;

pub fn build(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Widget {
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.add_css_class("page");

    // Title
    let title_box = Box::new(Orientation::Horizontal, 8);
    title_box.set_margin_start(24);
    title_box.set_margin_end(24);
    title_box.set_margin_top(20);
    title_box.set_margin_bottom(0);

    let title_lbl = Label::new(Some("Java Runtime"));
    title_lbl.add_css_class("title-1");
    title_lbl.set_hexpand(true);
    title_lbl.set_halign(gtk4::Align::Start);

    title_box.append(&title_lbl);
    vbox.append(&title_box);

    let sub_lbl = Label::new(Some("Manage Java installations used for launching Minecraft."));
    sub_lbl.add_css_class("dim-label");
    sub_lbl.set_halign(gtk4::Align::Start);
    sub_lbl.set_margin_start(24);
    sub_lbl.set_margin_end(24);
    sub_lbl.set_margin_top(4);
    vbox.append(&sub_lbl);

    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_start(24);
    sep.set_margin_end(24);
    sep.set_margin_top(12);
    sep.set_margin_bottom(8);
    vbox.append(&sep);

    // Download section
    let download_group = libadwaita::PreferencesGroup::new();
    download_group.set_title("Download Java");
    download_group.set_description(Some("Download Java versions via Eclipse Adoptium (Temurin)"));
    download_group.set_margin_start(24);
    download_group.set_margin_end(24);

    let versions = [
        (8u32,  "Java 8",  "Required for Minecraft 1.12 and below"),
        (11,    "Java 11", "Required for some older forge mods"),
        (17,    "Java 17", "Required for Minecraft 1.17 to 1.20"),
        (21,    "Java 21", "Recommended for Minecraft 1.20.5+"),
        (22,    "Java 22", "Latest stable"),
        (23,    "Java 23", "Latest (experimental)"),
        (24,    "Java 24", "Cutting edge"),
    ];

    for (major, name, desc) in versions {
        let row = libadwaita::ActionRow::new();
        row.set_title(name);
        row.set_subtitle(desc);

        let icon = Image::from_icon_name("emblem-system-symbolic");
        row.add_prefix(&icon);

        let btn_box = Box::new(Orientation::Horizontal, 4);
        btn_box.set_valign(gtk4::Align::Center);

        let check_btn = Button::new();
        check_btn.set_icon_name("object-select-symbolic");
        check_btn.add_css_class("circular");
        check_btn.set_tooltip_text(Some("Check if installed"));

        let state_c = state.clone();
        check_btn.connect_clicked(move |btn| {
            let java_mgr = crate::java::JavaManager::new(
                state_c.borrow().http_client.clone()
            );
            let installs = java_mgr.detect_system_java();
            let has_java = installs.iter().any(|j| j.major_version == major);

            let toast_msg = if has_java {
                format!("Java {major} is available!")
            } else {
                format!("Java {major} is NOT installed")
            };
            log::info!("{toast_msg}");
        });

        let dl_btn = Button::with_label("Download");
        dl_btn.add_css_class("suggested-action");
        dl_btn.set_tooltip_text(Some(&format!("Download Java {major} via Adoptium")));

        let state_c2 = state.clone();
        let window_c = window.clone();
        dl_btn.connect_clicked(move |btn| {
            btn.set_label("Downloading...");
            btn.set_sensitive(false);

            let http_client = state_c2.borrow().http_client.clone();
            let (tx, rx) = glib::MainContext::channel::<Result<String, String>>(glib::Priority::DEFAULT);
            let btn_clone = btn.clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let result = rt.block_on(async {
                    let mgr = crate::java::JavaManager::new(http_client);
                    mgr.download_java(major, None).await
                });
                let _ = tx.send(result.map(|j| format!("Downloaded Java {} ({})", major, j.version))
                    .map_err(|e| e.to_string()));
            });

            rx.attach(None, move |result| {
                btn_clone.set_label("Download");
                btn_clone.set_sensitive(true);
                match result {
                    Ok(msg) => log::info!("{msg}"),
                    Err(e) => log::error!("Java download failed: {e}"),
                }
                glib::ControlFlow::Break
            });
        });

        btn_box.append(&check_btn);
        btn_box.append(&dl_btn);
        row.add_suffix(&btn_box);
        download_group.add(&row);
    }

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let inner = Box::new(Orientation::Vertical, 16);
    inner.set_margin_top(8);
    inner.set_margin_bottom(24);

    // Detected installs
    let detected_group = libadwaita::PreferencesGroup::new();
    detected_group.set_title("Detected Installations");
    detected_group.set_margin_start(24);
    detected_group.set_margin_end(24);

    {
        let java_mgr = crate::java::JavaManager::new(state.borrow().http_client.clone());
        let installs = java_mgr.detect_system_java();

        if installs.is_empty() {
            let row = libadwaita::ActionRow::new();
            row.set_title("No Java installations detected");
            row.set_subtitle("Download one below or install via your package manager (pacman -S jre-openjdk)");
            detected_group.add(&row);
        } else {
            for install in installs {
                let row = libadwaita::ActionRow::new();
                row.set_title(&format!("Java {}", install.major_version));
                row.set_subtitle(&format!("{} — {}", install.version, install.path));

                let badge_lbl = Label::new(Some(if install.is_bundled { "Bundled" } else { "System" }));
                badge_lbl.add_css_class("tag");
                badge_lbl.set_valign(gtk4::Align::Center);
                row.add_suffix(&badge_lbl);

                detected_group.add(&row);
            }
        }
    }

    inner.append(&detected_group);
    inner.append(&download_group);
    scroll.set_child(Some(&inner));
    vbox.append(&scroll);
    vbox.into()
}
