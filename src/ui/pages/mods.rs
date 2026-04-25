use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, ScrolledWindow, Image, Entry, ComboBoxText};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use libadwaita::ApplicationWindow;

pub fn build(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Widget {
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.add_css_class("page");

    // Tab bar
    let tab_bar_box = Box::new(Orientation::Horizontal, 0);
    tab_bar_box.add_css_class("tab-bar-box");
    tab_bar_box.set_margin_start(24);
    tab_bar_box.set_margin_end(24);
    tab_bar_box.set_margin_top(20);
    tab_bar_box.set_margin_bottom(0);

    let title = Label::new(Some("Mods Browser"));
    title.add_css_class("title-1");
    title.set_hexpand(true);
    title.set_halign(gtk4::Align::Start);
    tab_bar_box.append(&title);

    vbox.append(&tab_bar_box);

    // Source selector
    let source_box = Box::new(Orientation::Horizontal, 8);
    source_box.set_margin_start(24);
    source_box.set_margin_end(24);
    source_box.set_margin_top(12);
    source_box.set_margin_bottom(4);

    let source_label = Label::new(Some("Source:"));
    source_label.add_css_class("body");

    let source_combo = ComboBoxText::new();
    source_combo.append(Some("modrinth"), "Modrinth");
    source_combo.append(Some("curseforge"), "CurseForge");
    source_combo.set_active_id(Some("modrinth"));

    let instance_label = Label::new(Some("Instance:"));
    instance_label.add_css_class("body");
    instance_label.set_margin_start(12);

    let instance_combo = ComboBoxText::new();
    instance_combo.append(Some("none"), "— Select instance —");

    {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        for inst in &instances.instances {
            instance_combo.append(Some(&inst.id), &inst.name);
        }
    }
    instance_combo.set_active_id(Some("none"));

    source_box.append(&source_label);
    source_box.append(&source_combo);
    source_box.append(&instance_label);
    source_box.append(&instance_combo);
    vbox.append(&source_box);

    // Search bar
    let search_box = Box::new(Orientation::Horizontal, 8);
    search_box.set_margin_start(24);
    search_box.set_margin_end(24);
    search_box.set_margin_top(8);
    search_box.set_margin_bottom(8);

    let search_entry = Entry::new();
    search_entry.set_placeholder_text(Some("Search mods..."));
    search_entry.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("system-search-symbolic"));
    search_entry.set_hexpand(true);

    let search_btn = Button::with_label("Search");
    search_btn.add_css_class("suggested-action");

    search_box.append(&search_entry);
    search_box.append(&search_btn);
    vbox.append(&search_box);

    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_start(24);
    sep.set_margin_end(24);
    vbox.append(&sep);

    // Results area
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let results_list = gtk4::ListBox::new();
    results_list.add_css_class("boxed-list");
    results_list.set_margin_start(24);
    results_list.set_margin_end(24);
    results_list.set_margin_top(8);
    results_list.set_margin_bottom(24);
    results_list.set_selection_mode(gtk4::SelectionMode::None);
    results_list.set_widget_name("mods-results-list");

    let placeholder = Label::new(Some("Search for mods above. Select an instance to download mods directly."));
    placeholder.add_css_class("dim-label");
    placeholder.set_margin_top(32);
    placeholder.set_wrap(true);
    placeholder.set_justify(gtk4::Justification::Center);
    results_list.append(&placeholder);

    scroll.set_child(Some(&results_list));
    vbox.append(&scroll);

    // Wire search
    let state_clone = state.clone();
    let results_list_clone = results_list.clone();
    let source_combo_clone = source_combo.clone();
    let instance_combo_clone = instance_combo.clone();
    let window_clone = window.clone();

    search_btn.connect_clicked(move |_| {
        let query = search_entry.text().to_string();
        let source = source_combo_clone.active_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "modrinth".to_string());
        let instance_id = instance_combo_clone.active_id()
            .filter(|id| *id != "none")
            .map(|id| id.to_string());

        let state = state_clone.clone();
        let results_list = results_list_clone.clone();
        let window = window_clone.clone();

        if source == "modrinth" {
            perform_modrinth_search(query, instance_id, state, results_list, window);
        } else {
            show_curseforge_api_key_notice(&results_list);
        }
    });

    vbox.upcast::<gtk4::Widget>()
}

fn perform_modrinth_search(
    query: String,
    instance_id: Option<String>,
    state: Rc<RefCell<AppState>>,
    results_list: gtk4::ListBox,
    window: ApplicationWindow,
) {
    // Clear results
    while let Some(child) = results_list.first_child() {
        results_list.remove(&child);
    }

    let loading = Label::new(Some("Searching..."));
    loading.add_css_class("dim-label");
    loading.set_margin_top(24);
    results_list.append(&loading);

    let http_client = state.borrow().http_client.clone();

    // Get mc version from selected instance
    let mc_version = instance_id.as_ref().and_then(|id| {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(id).map(|i| i.minecraft_version.clone())
    });

    let loader = instance_id.as_ref().and_then(|id| {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(id).map(|i| match i.mod_loader {
            crate::minecraft::instances::ModLoader::Vanilla => None,
            crate::minecraft::instances::ModLoader::Fabric => Some("fabric".to_string()),
            crate::minecraft::instances::ModLoader::Forge => Some("forge".to_string()),
            crate::minecraft::instances::ModLoader::Quilt => Some("quilt".to_string()),
            crate::minecraft::instances::ModLoader::NeoForge => Some("neoforge".to_string()),
        }).flatten()
    });

    let (tx, rx) = glib::MainContext::channel::<Result<Vec<crate::mods::modrinth::ModrinthProject>, String>>(
        glib::Priority::DEFAULT,
    );

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            let modrinth = crate::mods::modrinth::ModrinthClient::new(http_client);
            let search_query = crate::mods::modrinth::SearchQuery {
                query: query.clone(),
                mc_version: mc_version.clone(),
                loader: loader.clone(),
                ..Default::default()
            };
            modrinth.search(&search_query).await
        });

        let _ = tx.send(result.map(|r| r.hits).map_err(|e| e.to_string()));
    });

    let instance_id_clone = instance_id.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    rx.attach(None, move |result| {
        // Clear loading
        while let Some(child) = results_list.first_child() {
            results_list.remove(&child);
        }

        match result {
            Ok(hits) if hits.is_empty() => {
                let lbl = Label::new(Some("No results found"));
                lbl.add_css_class("dim-label");
                lbl.set_margin_top(24);
                results_list.append(&lbl);
            }
            Ok(hits) => {
                for project in hits {
                    let row = build_mod_row(&project, instance_id_clone.clone(), state_clone.clone(), &window_clone);
                    results_list.append(&row);
                }
            }
            Err(e) => {
                let lbl = Label::new(Some(&format!("Error: {e}")));
                lbl.add_css_class("error");
                lbl.set_margin_top(24);
                results_list.append(&lbl);
            }
        }

        glib::ControlFlow::Break
    });
}

fn build_mod_row(
    project: &crate::mods::modrinth::ModrinthProject,
    instance_id: Option<String>,
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) -> gtk4::Widget {
    let row = libadwaita::ActionRow::new();
    row.set_title(&project.title);
    row.set_subtitle(&format!(
        "⬇ {} downloads  |  {}",
        format_number(project.downloads),
        project.categories.first().map(|s| s.as_str()).unwrap_or("mod")
    ));

    // Icon
    let icon = Image::from_icon_name("package-x-generic-symbolic");
    icon.set_pixel_size(36);
    row.add_prefix(&icon);

    // Download button
    let dl_btn = Button::new();
    dl_btn.set_icon_name("folder-download-symbolic");
    dl_btn.add_css_class("circular");
    dl_btn.add_css_class("suggested-action");
    dl_btn.set_valign(gtk4::Align::Center);
    dl_btn.set_tooltip_text(Some("Download to instance"));

    let project_id = project.project_id.clone();
    let project_title = project.title.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    let instance_id_clone = instance_id.clone();

    dl_btn.connect_clicked(move |_| {
        if let Some(inst_id) = &instance_id_clone {
            download_mod_to_instance(
                project_id.clone(),
                project_title.clone(),
                inst_id.clone(),
                state_clone.clone(),
                &window_clone,
            );
        } else {
            let dialog = libadwaita::MessageDialog::new(
                Some(&window_clone),
                Some("No Instance Selected"),
                Some("Please select an instance in the dropdown above to download mods."),
            );
            dialog.add_response("ok", "OK");
            dialog.present();
        }
    });

    let info_btn = Button::new();
    info_btn.set_icon_name("help-about-symbolic");
    info_btn.add_css_class("circular");
    info_btn.set_valign(gtk4::Align::Center);
    info_btn.set_tooltip_text(Some("Open on Modrinth"));

    let slug = project.project_id.clone();
    info_btn.connect_clicked(move |_| {
        let _ = open::that(format!("https://modrinth.com/mod/{slug}"));
    });

    let btn_box = Box::new(Orientation::Horizontal, 4);
    btn_box.set_valign(gtk4::Align::Center);
    btn_box.append(&info_btn);
    btn_box.append(&dl_btn);
    row.add_suffix(&btn_box);

    row.upcast::<gtk4::Widget>()
}

fn download_mod_to_instance(
    project_id: String,
    _project_title: String,
    instance_id: String,
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) {
    let mods_dir = {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(&instance_id).map(|i| i.mods_dir())
    };

    let Some(mods_dir) = mods_dir else {
        return;
    };

    let mc_version = {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(&instance_id).map(|i| i.minecraft_version.clone())
    };

    let loader = {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(&instance_id).map(|i| match i.mod_loader {
            crate::minecraft::instances::ModLoader::Vanilla => None,
            crate::minecraft::instances::ModLoader::Fabric => Some("fabric"),
            crate::minecraft::instances::ModLoader::Forge => Some("forge"),
            crate::minecraft::instances::ModLoader::Quilt => Some("quilt"),
            crate::minecraft::instances::ModLoader::NeoForge => Some("neoforge"),
        }).flatten().map(|s| s.to_string())
    };

    let http_client = state.borrow().http_client.clone();

    let (tx, rx) = glib::MainContext::channel::<Result<String, String>>(glib::Priority::DEFAULT);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = rt.block_on(async {
            let modrinth = crate::mods::modrinth::ModrinthClient::new(http_client);
            let versions = modrinth
                .get_project_versions(&project_id, mc_version.as_deref(), loader.as_deref())
                .await?;

            let version = versions.first()
                .ok_or_else(|| anyhow::anyhow!("No compatible version found"))?;

            let path = modrinth.download_mod(version, &mods_dir, None).await?;
            Ok::<_, anyhow::Error>(path.file_name().unwrap_or_default().to_string_lossy().to_string())
        });

        let _ = tx.send(result.map_err(|e| e.to_string()));
    });

    let window_clone2 = window.clone();
    rx.attach(None, move |result| {
        let msg = match result {
            Ok(filename) => format!("Downloaded: {filename}"),
            Err(e) => format!("Download failed: {e}"),
        };
        log::info!("{msg}");
        let dialog = libadwaita::MessageDialog::new(
            Some(&window_clone2),
            Some("Mod Download"),
            Some(&msg),
        );
        dialog.add_response("ok", "OK");
        dialog.present();
        glib::ControlFlow::Break
    });
}

fn show_curseforge_api_key_notice(results_list: &gtk4::ListBox) {
    while let Some(child) = results_list.first_child() {
        results_list.remove(&child);
    }
    let lbl = Label::new(Some(
        "CurseForge requires an API key.\nPlease add your CurseForge API key in Settings > CurseForge API Key.",
    ));
    lbl.add_css_class("dim-label");
    lbl.set_margin_top(24);
    lbl.set_wrap(true);
    lbl.set_justify(gtk4::Justification::Center);
    results_list.append(&lbl);
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
