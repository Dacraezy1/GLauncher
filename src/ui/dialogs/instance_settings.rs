use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, Entry, ScrolledWindow};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use libadwaita::ApplicationWindow;

pub fn show(instance_id: &str, state: Rc<RefCell<AppState>>, window: &ApplicationWindow) {
    let instance = {
        let st = state.borrow();
        let instances = st.instances.lock().unwrap();
        instances.get(instance_id).cloned()
    };

    let Some(instance) = instance else { return; };

    let dialog = libadwaita::Window::new();
    dialog.set_title(Some(&format!("Settings — {}", instance.name)));
    dialog.set_transient_for(Some(window));
    dialog.set_modal(true);
    dialog.set_default_size(560, 680);

    let content = Box::new(Orientation::Vertical, 0);

    let header = libadwaita::HeaderBar::new();
    header.set_show_end_title_buttons(false);

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("flat");
    header.pack_start(&cancel_btn);

    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    header.pack_end(&save_btn);

    content.append(&header);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let inner = Box::new(Orientation::Vertical, 16);
    inner.set_margin_start(20);
    inner.set_margin_end(20);
    inner.set_margin_top(12);
    inner.set_margin_bottom(20);

    // ── General ───────────────────────────────────────────────────────
    let general_group = libadwaita::PreferencesGroup::new();
    general_group.set_title("General");

    let name_row = libadwaita::EntryRow::new();
    name_row.set_title("Instance Name");
    name_row.set_text(&instance.name);
    general_group.add(&name_row);

    let notes_row = libadwaita::EntryRow::new();
    notes_row.set_title("Notes");
    notes_row.set_text(&instance.notes);
    general_group.add(&notes_row);

    inner.append(&general_group);

    // ── JVM Memory ────────────────────────────────────────────────────
    let jvm_group = libadwaita::PreferencesGroup::new();
    jvm_group.set_title("JVM Memory");

    let xms_row = libadwaita::ActionRow::new();
    xms_row.set_title("Minimum Memory (Xms)");
    xms_row.set_subtitle("MB");
    let xms_spin = gtk4::SpinButton::with_range(256.0, 16384.0, 128.0);
    xms_spin.set_value(instance.jvm_settings.memory_min_mb as f64);
    xms_spin.set_valign(gtk4::Align::Center);
    xms_row.add_suffix(&xms_spin);
    jvm_group.add(&xms_row);

    let xmx_row = libadwaita::ActionRow::new();
    xmx_row.set_title("Maximum Memory (Xmx)");
    xmx_row.set_subtitle("MB");
    let xmx_spin = gtk4::SpinButton::with_range(512.0, 65536.0, 256.0);
    xmx_spin.set_value(instance.jvm_settings.memory_max_mb as f64);
    xmx_spin.set_valign(gtk4::Align::Center);
    xmx_row.add_suffix(&xmx_spin);
    jvm_group.add(&xmx_row);

    inner.append(&jvm_group);

    // ── GC Tuning ─────────────────────────────────────────────────────
    let gc_group = libadwaita::PreferencesGroup::new();
    gc_group.set_title("GC / Performance Flags");
    gc_group.set_description(Some("Choose a garbage collector tuning preset"));

    let gc_combo_row = libadwaita::ComboRow::new();
    gc_combo_row.set_title("GC Preset");
    let gc_model = gtk4::StringList::new(&[
        "G1GC (default)",
        "Aikar's Flags (G1GC, max performance)",
        "ZGC (low latency)",
        "Shenandoah (ultra-low pause)",
        "No preset",
    ]);
    gc_combo_row.set_model(Some(&gc_model));

    let gc_selected = if instance.jvm_settings.use_aikar_flags { 1 }
        else if instance.jvm_settings.use_zgc { 2 }
        else if instance.jvm_settings.use_shenandoah { 3 }
        else if instance.jvm_settings.use_g1gc { 0 }
        else { 4 };
    gc_combo_row.set_selected(gc_selected);
    gc_group.add(&gc_combo_row);

    inner.append(&gc_group);

    // ── Extra JVM Args ────────────────────────────────────────────────
    let extra_group = libadwaita::PreferencesGroup::new();
    extra_group.set_title("Extra JVM Arguments");
    extra_group.set_description(Some("One argument per line, e.g. -Dfoo=bar"));

    let extra_row = libadwaita::ActionRow::new();
    let extra_view = gtk4::TextView::new();
    extra_view.set_monospace(true);
    extra_view.set_wrap_mode(gtk4::WrapMode::Word);
    extra_view.set_size_request(-1, 80);
    extra_view.set_margin_top(4);
    extra_view.set_margin_bottom(4);
    let extra_scroll = gtk4::ScrolledWindow::new();
    extra_scroll.set_child(Some(&extra_view));
    extra_scroll.set_hexpand(true);

    let buffer = extra_view.buffer();
    buffer.set_text(&instance.jvm_settings.extra_jvm_args.join("\n"));
    extra_row.set_child(Some(&extra_scroll));
    extra_group.add(&extra_row);

    inner.append(&extra_group);

    // ── Java ─────────────────────────────────────────────────────────
    let java_group = libadwaita::PreferencesGroup::new();
    java_group.set_title("Java Installation");

    let java_row = libadwaita::EntryRow::new();
    java_row.set_title("Java Path (leave blank for auto)");
    if let Some(p) = &instance.jvm_settings.java_path {
        java_row.set_text(p);
    }
    java_group.add(&java_row);

    inner.append(&java_group);

    // ── Window ────────────────────────────────────────────────────────
    let win_group = libadwaita::PreferencesGroup::new();
    win_group.set_title("Game Window");

    let fullscreen_row = libadwaita::SwitchRow::new();
    fullscreen_row.set_title("Fullscreen");
    fullscreen_row.set_active(instance.fullscreen);
    win_group.add(&fullscreen_row);

    let auto_res_row = libadwaita::SwitchRow::new();
    auto_res_row.set_title("Auto Resolution");
    auto_res_row.set_subtitle("Let Minecraft decide window size");
    auto_res_row.set_active(instance.game_resolution_auto);
    win_group.add(&auto_res_row);

    let width_row = libadwaita::ActionRow::new();
    width_row.set_title("Width");
    let width_spin = gtk4::SpinButton::with_range(640.0, 7680.0, 1.0);
    width_spin.set_value(instance.window_width.unwrap_or(854) as f64);
    width_spin.set_valign(gtk4::Align::Center);
    width_row.add_suffix(&width_spin);
    win_group.add(&width_row);

    let height_row = libadwaita::ActionRow::new();
    height_row.set_title("Height");
    let height_spin = gtk4::SpinButton::with_range(480.0, 4320.0, 1.0);
    height_spin.set_value(instance.window_height.unwrap_or(480) as f64);
    height_spin.set_valign(gtk4::Align::Center);
    height_row.add_suffix(&height_spin);
    win_group.add(&height_row);

    inner.append(&win_group);

    scroll.set_child(Some(&inner));
    content.append(&scroll);
    dialog.set_content(Some(&content));

    // Wire cancel
    let dialog_c = dialog.clone();
    cancel_btn.connect_clicked(move |_| dialog_c.close());

    // Wire save
    let dialog_c = dialog.clone();
    let state_c = state.clone();
    let inst_id = instance.id.clone();

    save_btn.connect_clicked(move |_| {
        let mut st = state_c.borrow_mut();
        let mut instances = st.instances.lock().unwrap();

        if let Some(inst) = instances.get_mut(&inst_id) {
            inst.name = name_row.text().to_string();
            inst.notes = notes_row.text().to_string();
            inst.jvm_settings.memory_min_mb = xms_spin.value() as u32;
            inst.jvm_settings.memory_max_mb = xmx_spin.value() as u32;
            inst.fullscreen = fullscreen_row.is_active();
            inst.game_resolution_auto = auto_res_row.is_active();
            inst.window_width = Some(width_spin.value() as u32);
            inst.window_height = Some(height_spin.value() as u32);

            // GC preset
            let gc_sel = gc_combo_row.selected();
            inst.jvm_settings.use_aikar_flags = gc_sel == 1;
            inst.jvm_settings.use_zgc = gc_sel == 2;
            inst.jvm_settings.use_shenandoah = gc_sel == 3;
            inst.jvm_settings.use_g1gc = gc_sel == 0;

            // Extra args
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, false).to_string();
            inst.jvm_settings.extra_jvm_args = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            let java_text = java_row.text().to_string();
            inst.jvm_settings.java_path = if java_text.is_empty() { None } else { Some(java_text) };

            let _ = inst.save();
        }

        dialog_c.close();
    });

    dialog.present();
}
