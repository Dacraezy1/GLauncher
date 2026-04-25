use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, Entry, ComboBoxText, ScrolledWindow, ListBox, ListBoxRow};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use crate::minecraft::instances::{Instance, ModLoader};
use libadwaita::ApplicationWindow;

pub fn show(state: &Rc<RefCell<AppState>>, window: &ApplicationWindow) {
    let dialog = libadwaita::Window::new();
    dialog.set_title(Some("New Instance"));
    dialog.set_transient_for(Some(window));
    dialog.set_modal(true);
    dialog.set_default_size(520, 560);
    dialog.set_resizable(false);

    let content = Box::new(Orientation::Vertical, 0);

    // Header bar
    let header = libadwaita::HeaderBar::new();
    header.set_show_end_title_buttons(false);

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("flat");
    header.pack_start(&cancel_btn);

    let create_btn = Button::with_label("Create");
    create_btn.add_css_class("suggested-action");
    header.pack_end(&create_btn);

    content.append(&header);

    let inner = Box::new(Orientation::Vertical, 16);
    inner.set_margin_start(20);
    inner.set_margin_end(20);
    inner.set_margin_top(12);
    inner.set_margin_bottom(20);

    // Name
    let name_group = libadwaita::PreferencesGroup::new();
    name_group.set_title("Instance Name");
    let name_entry = libadwaita::EntryRow::new();
    name_entry.set_title("Name");
    name_entry.set_text("My Instance");
    name_group.add(&name_entry);
    inner.append(&name_group);

    // Mod loader selection
    let loader_group = libadwaita::PreferencesGroup::new();
    loader_group.set_title("Mod Loader");

    let loader_combo_row = libadwaita::ComboRow::new();
    loader_combo_row.set_title("Loader");

    let loader_model = gtk4::StringList::new(&["Vanilla", "Fabric", "Forge", "Quilt", "NeoForge"]);
    loader_combo_row.set_model(Some(&loader_model));
    loader_combo_row.set_selected(0);

    loader_group.add(&loader_combo_row);

    // Loader version row (hidden for Vanilla)
    let loader_version_row = libadwaita::ActionRow::new();
    loader_version_row.set_title("Loader Version");
    let loader_version_combo = ComboBoxText::new();
    loader_version_combo.append(Some("latest"), "Latest (auto)");
    loader_version_combo.set_active_id(Some("latest"));
    loader_version_combo.set_valign(gtk4::Align::Center);
    loader_version_row.add_suffix(&loader_version_combo);
    loader_version_row.set_visible(false);
    loader_group.add(&loader_version_row);

    loader_combo_row.connect_selected_notify({
        let loader_version_row = loader_version_row.clone();
        move |row| {
            let is_vanilla = row.selected() == 0;
            loader_version_row.set_visible(!is_vanilla);
        }
    });

    inner.append(&loader_group);

    // Minecraft version
    let version_group = libadwaita::PreferencesGroup::new();
    version_group.set_title("Minecraft Version");

    let version_combo_row = libadwaita::ActionRow::new();
    version_combo_row.set_title("Version");

    let mc_combo = ComboBoxText::new();
    mc_combo.append(Some("loading"), "Loading versions...");
    mc_combo.set_active_id(Some("loading"));
    mc_combo.set_hexpand(true);
    mc_combo.set_valign(gtk4::Align::Center);
    version_combo_row.add_suffix(&mc_combo);
    version_group.add(&version_combo_row);

    inner.append(&version_group);

    // JVM Memory quick setting
    let jvm_group = libadwaita::PreferencesGroup::new();
    jvm_group.set_title("Memory (quick)");

    let mem_row = libadwaita::ActionRow::new();
    mem_row.set_title("RAM (MB)");
    mem_row.set_subtitle("Maximum memory allocated to Minecraft");

    let mem_spin = gtk4::SpinButton::with_range(512.0, 32768.0, 512.0);
    mem_spin.set_value(2048.0);
    mem_spin.set_valign(gtk4::Align::Center);
    mem_row.add_suffix(&mem_spin);
    jvm_group.add(&mem_row);

    inner.append(&jvm_group);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroll.set_child(Some(&inner));
    content.append(&scroll);

    dialog.set_content(Some(&content));

    // Load versions async
    let http_client = state.borrow().http_client.clone();
    let mc_combo_clone = mc_combo.clone();
    let config = state.borrow().config.lock().unwrap().clone();

    let (tx, rx) = glib::MainContext::channel::<Result<Vec<crate::minecraft::versions::VersionEntry>, String>>(
        glib::Priority::DEFAULT,
    );

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(async {
            let vm = crate::minecraft::versions::VersionManager::new(http_client);
            vm.fetch_manifest().await
        });
        let _ = tx.send(result.map(|m| m.versions).map_err(|e| e.to_string()));
    });

    rx.attach(None, move |result| {
        mc_combo_clone.remove_all();
        match result {
            Ok(versions) => {
                for v in &versions {
                    let show = match v.version_type {
                        crate::minecraft::versions::VersionType::Release => true,
                        crate::minecraft::versions::VersionType::Snapshot => config.show_snapshots,
                        crate::minecraft::versions::VersionType::OldBeta => config.show_beta,
                        crate::minecraft::versions::VersionType::OldAlpha => config.show_alpha,
                    };
                    if show {
                        mc_combo_clone.append(Some(&v.id), &format!("{} ({})", v.id, v.version_type));
                    }
                }
                mc_combo_clone.set_active(Some(0));
            }
            Err(e) => {
                mc_combo_clone.append(Some("error"), &format!("Error: {e}"));
                mc_combo_clone.set_active(Some(0));
            }
        }
        glib::ControlFlow::Break
    });

    // Wire cancel
    let dialog_c = dialog.clone();
    cancel_btn.connect_clicked(move |_| dialog_c.close());

    // Wire create
    let dialog_c = dialog.clone();
    let state_c = state.clone();
    let window_c = window.clone();
    create_btn.connect_clicked(move |_| {
        let name = name_entry.text().to_string().trim().to_string();
        if name.is_empty() {
            return;
        }

        let mc_version = mc_combo.active_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "1.21.4".to_string());

        if mc_version == "loading" || mc_version == "error" {
            return;
        }

        let loader_idx = loader_combo_row.selected();
        let mod_loader = match loader_idx {
            0 => ModLoader::Vanilla,
            1 => ModLoader::Fabric,
            2 => ModLoader::Forge,
            3 => ModLoader::Quilt,
            4 => ModLoader::NeoForge,
            _ => ModLoader::Vanilla,
        };

        let memory_max = mem_spin.value() as u32;

        let mut instance = Instance::new(&name, &mc_version, mod_loader);
        instance.jvm_settings.memory_max_mb = memory_max;
        instance.jvm_settings.memory_min_mb = (memory_max / 4).max(256);

        {
            let st = state_c.borrow();
            let mut instances = st.instances.lock().unwrap();
            let _ = instances.add(instance);
        }

        dialog_c.close();
    });

    dialog.present();
}
