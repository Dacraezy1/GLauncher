use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, ScrolledWindow, Switch, SpinButton, Adjustment};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use libadwaita::ApplicationWindow;

pub fn build(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Widget {
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.add_css_class("page");

    let title_lbl = Label::new(Some("Settings"));
    title_lbl.add_css_class("title-1");
    title_lbl.set_halign(gtk4::Align::Start);
    title_lbl.set_margin_start(24);
    title_lbl.set_margin_end(24);
    title_lbl.set_margin_top(20);
    vbox.append(&title_lbl);

    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_start(24);
    sep.set_margin_end(24);
    sep.set_margin_top(12);
    sep.set_margin_bottom(4);
    vbox.append(&sep);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let inner = Box::new(Orientation::Vertical, 20);
    inner.set_margin_start(24);
    inner.set_margin_end(24);
    inner.set_margin_top(12);
    inner.set_margin_bottom(24);

    let cfg = state.borrow().config.lock().unwrap().clone();

    // ── Appearance ─────────────────────────────────────────────────────
    let appearance_group = libadwaita::PreferencesGroup::new();
    appearance_group.set_title("Appearance");

    // Color scheme
    let theme_row = libadwaita::ActionRow::new();
    theme_row.set_title("Color Scheme");
    theme_row.set_subtitle("Choose light, dark, or follow system preference");
    let theme_combo = gtk4::ComboBoxText::new();
    theme_combo.append(Some("system"), "System Default");
    theme_combo.append(Some("light"), "Light");
    theme_combo.append(Some("dark"), "Dark");
    theme_combo.set_active_id(Some(&cfg.theme));
    theme_combo.set_valign(gtk4::Align::Center);

    let state_c = state.clone();
    theme_combo.connect_changed(move |combo| {
        if let Some(id) = combo.active_id() {
            let mut st = state_c.borrow_mut();
            let mut config = st.config.lock().unwrap();
            config.theme = id.to_string();
            let _ = config.save();

            // Apply immediately
            let style_mgr = libadwaita::StyleManager::default();
            match id.as_str() {
                "light" => style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceLight),
                "dark"  => style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark),
                _       => style_mgr.set_color_scheme(libadwaita::ColorScheme::Default),
            }
        }
    });

    theme_row.add_suffix(&theme_combo);
    appearance_group.add(&theme_row);

    inner.append(&appearance_group);

    // ── Version Visibility ─────────────────────────────────────────────
    let versions_group = libadwaita::PreferencesGroup::new();
    versions_group.set_title("Version Display");
    versions_group.set_description(Some("Control which Minecraft versions are shown when creating instances"));

    add_switch_row(
        &versions_group,
        "Show Snapshots",
        "Show weekly snapshot builds",
        cfg.show_snapshots,
        {
            let state_c = state.clone();
            move |v| {
                let mut st = state_c.borrow_mut();
                let mut config = st.config.lock().unwrap();
                config.show_snapshots = v;
                let _ = config.save();
            }
        },
    );

    add_switch_row(
        &versions_group,
        "Show Beta",
        "Show old beta versions (b1.x)",
        cfg.show_beta,
        {
            let state_c = state.clone();
            move |v| {
                let mut st = state_c.borrow_mut();
                let mut config = st.config.lock().unwrap();
                config.show_beta = v;
                let _ = config.save();
            }
        },
    );

    add_switch_row(
        &versions_group,
        "Show Alpha",
        "Show old alpha versions (a1.x)",
        cfg.show_alpha,
        {
            let state_c = state.clone();
            move |v| {
                let mut st = state_c.borrow_mut();
                let mut config = st.config.lock().unwrap();
                config.show_alpha = v;
                let _ = config.save();
            }
        },
    );

    inner.append(&versions_group);

    // ── Launcher Behaviour ─────────────────────────────────────────────
    let launcher_group = libadwaita::PreferencesGroup::new();
    launcher_group.set_title("Launcher Behaviour");

    add_switch_row(
        &launcher_group,
        "Close on Launch",
        "Close GLauncher window when Minecraft starts",
        cfg.close_on_launch,
        {
            let state_c = state.clone();
            move |v| {
                let mut st = state_c.borrow_mut();
                let mut config = st.config.lock().unwrap();
                config.close_on_launch = v;
                let _ = config.save();
            }
        },
    );

    // Concurrent downloads
    let dl_row = libadwaita::ActionRow::new();
    dl_row.set_title("Concurrent Downloads");
    dl_row.set_subtitle("Number of simultaneous file downloads (1–16)");

    let dl_spin = SpinButton::with_range(1.0, 16.0, 1.0);
    dl_spin.set_value(cfg.concurrent_downloads as f64);
    dl_spin.set_valign(gtk4::Align::Center);

    let state_c = state.clone();
    dl_spin.connect_value_changed(move |spin| {
        let mut st = state_c.borrow_mut();
        let mut config = st.config.lock().unwrap();
        config.concurrent_downloads = spin.value() as u32;
        let _ = config.save();
    });

    dl_row.add_suffix(&dl_spin);
    launcher_group.add(&dl_row);

    inner.append(&launcher_group);

    // ── Default JVM ────────────────────────────────────────────────────
    let jvm_group = libadwaita::PreferencesGroup::new();
    jvm_group.set_title("Default JVM Settings");
    jvm_group.set_description(Some("These defaults apply to all new instances. You can override them per-instance."));

    let min_mem_row = libadwaita::ActionRow::new();
    min_mem_row.set_title("Minimum Memory (MB)");
    min_mem_row.set_subtitle("Xms value");
    let min_spin = SpinButton::with_range(256.0, 16384.0, 128.0);
    min_spin.set_value(cfg.default_memory_min as f64);
    min_spin.set_valign(gtk4::Align::Center);
    let state_c = state.clone();
    min_spin.connect_value_changed(move |s| {
        let mut st = state_c.borrow_mut();
        let mut cfg = st.config.lock().unwrap();
        cfg.default_memory_min = s.value() as u32;
        let _ = cfg.save();
    });
    min_mem_row.add_suffix(&min_spin);
    jvm_group.add(&min_mem_row);

    let max_mem_row = libadwaita::ActionRow::new();
    max_mem_row.set_title("Maximum Memory (MB)");
    max_mem_row.set_subtitle("Xmx value");
    let max_spin = SpinButton::with_range(512.0, 65536.0, 256.0);
    max_spin.set_value(cfg.default_memory_max as f64);
    max_spin.set_valign(gtk4::Align::Center);
    let state_c = state.clone();
    max_spin.connect_value_changed(move |s| {
        let mut st = state_c.borrow_mut();
        let mut cfg = st.config.lock().unwrap();
        cfg.default_memory_max = s.value() as u32;
        let _ = cfg.save();
    });
    max_mem_row.add_suffix(&max_spin);
    jvm_group.add(&max_mem_row);

    inner.append(&jvm_group);

    // ── CurseForge ────────────────────────────────────────────────────
    let cf_group = libadwaita::PreferencesGroup::new();
    cf_group.set_title("CurseForge API Key");
    cf_group.set_description(Some("Get your free key at https://console.curseforge.com"));

    let cf_row = libadwaita::ActionRow::new();
    cf_row.set_title("API Key");
    let cf_entry = gtk4::PasswordEntry::new();
    cf_entry.set_hexpand(true);
    cf_entry.set_placeholder_text(Some("$2a$10$..."));
    cf_entry.set_valign(gtk4::Align::Center);
    cf_row.add_suffix(&cf_entry);
    cf_group.add(&cf_row);

    inner.append(&cf_group);

    // ── About ──────────────────────────────────────────────────────────
    let about_group = libadwaita::PreferencesGroup::new();
    about_group.set_title("About");

    let about_row = libadwaita::ActionRow::new();
    about_row.set_title("GLauncher");
    about_row.set_subtitle("v1.0.0 — GPLv3 Open Source");

    let gh_btn = Button::with_label("GitHub");
    gh_btn.add_css_class("pill");
    gh_btn.set_valign(gtk4::Align::Center);
    gh_btn.connect_clicked(|_| {
        let _ = open::that("https://github.com/Dacraezy1/GLauncher");
    });
    about_row.add_suffix(&gh_btn);
    about_group.add(&about_row);

    inner.append(&about_group);

    scroll.set_child(Some(&inner));
    vbox.append(&scroll);
    vbox.into()
}

fn add_switch_row(
    group: &libadwaita::PreferencesGroup,
    title: &str,
    subtitle: &str,
    initial: bool,
    on_change: impl Fn(bool) + 'static,
) {
    let row = libadwaita::SwitchRow::new();
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.set_active(initial);
    row.connect_active_notify(move |r| on_change(r.is_active()));
    group.add(&row);
}
