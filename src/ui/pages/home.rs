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

    // Header bar
    let header = build_page_header("Home", "Your instances at a glance");
    vbox.append(&header);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let content = Box::new(Orientation::Vertical, 24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(16);
    content.set_margin_bottom(24);

    // Quick play: recent instances
    let recent_section = build_recent_instances_section(state.clone(), window);
    content.append(&recent_section);

    // News / welcome banner
    let banner = build_welcome_banner();
    content.append(&banner);

    scroll.set_child(Some(&content));
    vbox.append(&scroll);
    vbox.upcast::<gtk4::Widget>()
}

fn build_welcome_banner() -> gtk4::Widget {
    let card = gtk4::Box::new(Orientation::Vertical, 12);
    card.add_css_class("card");
    card.add_css_class("welcome-card");
    card.set_margin_top(8);

    let header = Label::new(Some("Welcome to GLauncher"));
    header.add_css_class("title-2");
    header.set_halign(gtk4::Align::Start);
    header.set_margin_start(16);
    header.set_margin_top(16);

    let desc = Label::new(Some(
        "A modern, open-source Minecraft Java Edition launcher.\n\
        Create instances, manage mods, and play with friends — all in one place."
    ));
    desc.add_css_class("body");
    desc.set_halign(gtk4::Align::Start);
    desc.set_margin_start(16);
    desc.set_margin_end(16);
    desc.set_wrap(true);

    let links_box = Box::new(Orientation::Horizontal, 8);
    links_box.set_margin_start(16);
    links_box.set_margin_bottom(16);

    let github_btn = Button::with_label("GitHub");
    github_btn.add_css_class("pill");
    github_btn.add_css_class("suggested-action");
    github_btn.connect_clicked(|_| {
        let _ = open::that("https://github.com/Dacraezy1/GLauncher");
    });

    let report_btn = Button::with_label("Report Bug");
    report_btn.add_css_class("pill");
    report_btn.connect_clicked(|_| {
        let _ = open::that("https://github.com/Dacraezy1/GLauncher/issues");
    });

    links_box.append(&github_btn);
    links_box.append(&report_btn);

    card.append(&header);
    card.append(&desc);
    card.append(&links_box);

    card.upcast::<gtk4::Widget>()
}

fn build_recent_instances_section(
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) -> gtk4::Widget {
    let section = Box::new(Orientation::Vertical, 12);

    let title = Label::new(Some("Recent Instances"));
    title.add_css_class("title-3");
    title.set_halign(gtk4::Align::Start);
    section.append(&title);

    let instances_box = Box::new(Orientation::Vertical, 8);
    instances_box.set_widget_name("home-instances-list");

    let st = state.borrow();
    let instances = st.instances.lock().unwrap();

    if instances.instances.is_empty() {
        let empty = Label::new(Some("No instances yet. Go to Instances to create one!"));
        empty.add_css_class("dim-label");
        empty.set_margin_top(16);
        instances_box.append(&empty);
    } else {
        let recent: Vec<_> = instances.instances.iter().take(5).collect();
        for inst in recent {
            let row = build_instance_quick_row(inst, state.clone(), window);
            instances_box.append(&row);
        }
    }

    section.append(&instances_box);
    section.upcast::<gtk4::Widget>()
}

fn build_instance_quick_row(
    inst: &crate::minecraft::instances::Instance,
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) -> gtk4::Widget {
    let card = Box::new(Orientation::Horizontal, 12);
    card.add_css_class("card");
    card.add_css_class("instance-quick-row");
    card.set_margin_top(2);

    let icon = Image::from_icon_name("applications-games-symbolic");
    icon.set_pixel_size(40);
    icon.add_css_class("instance-icon");
    icon.set_margin_start(12);

    let info = Box::new(Orientation::Vertical, 4);
    info.set_hexpand(true);
    info.set_valign(gtk4::Align::Center);

    let name_lbl = Label::new(Some(&inst.name));
    name_lbl.add_css_class("title-4");
    name_lbl.set_halign(gtk4::Align::Start);

    let version_lbl = Label::new(Some(&format!(
        "{} • {}",
        inst.minecraft_version,
        inst.loader_display()
    )));
    version_lbl.add_css_class("caption");
    version_lbl.add_css_class("dim-label");
    version_lbl.set_halign(gtk4::Align::Start);

    info.append(&name_lbl);
    info.append(&version_lbl);

    let play_btn = Button::new();
    play_btn.add_css_class("suggested-action");
    play_btn.add_css_class("circular");
    play_btn.set_icon_name("media-playback-start-symbolic");
    play_btn.set_valign(gtk4::Align::Center);
    play_btn.set_margin_end(12);
    play_btn.set_tooltip_text(Some("Launch"));

    let inst_id = inst.id.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    play_btn.connect_clicked(move |_| {
        crate::ui::dialogs::launch::launch_instance(
            &inst_id,
            state_clone.clone(),
            &window_clone,
        );
    });

    card.append(&icon);
    card.append(&info);
    card.append(&play_btn);

    card.upcast::<gtk4::Widget>()
}

pub fn build_page_header(title: &str, subtitle: &str) -> gtk4::Widget {
    let header = Box::new(Orientation::Vertical, 4);
    header.add_css_class("page-header");
    header.set_margin_start(24);
    header.set_margin_end(24);
    header.set_margin_top(20);
    header.set_margin_bottom(12);

    let title_lbl = Label::new(Some(title));
    title_lbl.add_css_class("title-1");
    title_lbl.set_halign(gtk4::Align::Start);

    let sub_lbl = Label::new(Some(subtitle));
    sub_lbl.add_css_class("body");
    sub_lbl.add_css_class("dim-label");
    sub_lbl.set_halign(gtk4::Align::Start);

    header.append(&title_lbl);
    header.append(&sub_lbl);

    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_top(12);
    header.append(&sep);

    header.upcast::<gtk4::Widget>()
}
