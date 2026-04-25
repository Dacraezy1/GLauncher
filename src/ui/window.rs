use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, HeaderBar, NavigationView, NavigationPage};
use gtk4::{Box, Orientation, Stack, StackSwitcher, ScrolledWindow, Label, Button, Image};
use std::rc::Rc;
use std::cell::RefCell;

use crate::ui::state::AppState;
use crate::ui::pages;

pub fn build_ui(app: &Application) {
    let state = match AppState::new() {
        Ok(s) => Rc::new(RefCell::new(s)),
        Err(e) => {
            eprintln!("Failed to initialize app state: {e}");
            let dialog = libadwaita::MessageDialog::new(
                None::<&ApplicationWindow>,
                Some("Initialization Error"),
                Some(&format!("Failed to start GLauncher: {e}")),
            );
            dialog.add_response("ok", "OK");
            dialog.present();
            return;
        }
    };

    let window = ApplicationWindow::builder()
        .application(app)
        .title("GLauncher")
        .default_width(1100)
        .default_height(700)
        .width_request(800)
        .height_request(550)
        .build();

    // Main layout: sidebar + content
    let split = gtk4::Paned::new(Orientation::Horizontal);
    split.set_shrink_start_child(false);
    split.set_shrink_end_child(false);
    split.set_position(220);

    // ── Sidebar ─────────────────────────────────────────────────────────
    let sidebar = build_sidebar(app, state.clone(), &window);
    split.set_start_child(Some(&sidebar));

    // ── Content stack ────────────────────────────────────────────────────
    let stack = Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_transition_duration(150);

    let home_page = pages::home::build(state.clone(), &window);
    let instances_page = pages::instances::build(state.clone(), &window);
    let mods_page = pages::mods::build(state.clone(), &window);
    let java_page = pages::java::build(state.clone(), &window);
    let accounts_page = pages::accounts::build(state.clone(), &window);
    let settings_page = pages::settings::build(state.clone(), &window);

    stack.add_named(&home_page, Some("home"));
    stack.add_named(&instances_page, Some("instances"));
    stack.add_named(&mods_page, Some("mods"));
    stack.add_named(&java_page, Some("java"));
    stack.add_named(&accounts_page, Some("accounts"));
    stack.add_named(&settings_page, Some("settings"));

    split.set_end_child(Some(&stack));
    window.set_content(Some(&split));

    // Wire sidebar buttons → stack
    wire_sidebar_to_stack(&sidebar, &stack);

    window.present();
}

fn build_sidebar(
    app: &Application,
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) -> gtk4::Box {
    let sidebar = gtk4::Box::new(Orientation::Vertical, 0);
    sidebar.add_css_class("sidebar");
    sidebar.set_width_request(220);

    // Logo area
    let logo_box = gtk4::Box::new(Orientation::Horizontal, 8);
    logo_box.add_css_class("sidebar-logo");
    logo_box.set_margin_start(16);
    logo_box.set_margin_end(16);
    logo_box.set_margin_top(20);
    logo_box.set_margin_bottom(8);

    let logo_icon = Image::from_icon_name("applications-games-symbolic");
    logo_icon.set_pixel_size(32);
    logo_icon.add_css_class("logo-icon");

    let logo_label = Label::new(Some("GLauncher"));
    logo_label.add_css_class("logo-label");
    logo_label.set_halign(gtk4::Align::Start);

    logo_box.append(&logo_icon);
    logo_box.append(&logo_label);

    sidebar.append(&logo_box);

    // Separator
    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_start(12);
    sep.set_margin_end(12);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    sidebar.append(&sep);

    // Nav buttons
    let nav_list = gtk4::ListBox::new();
    nav_list.add_css_class("navigation-sidebar");
    nav_list.set_selection_mode(gtk4::SelectionMode::Single);
    nav_list.set_margin_start(8);
    nav_list.set_margin_end(8);
    nav_list.set_margin_top(4);
    nav_list.set_vexpand(true);

    let nav_items = vec![
        ("home", "go-home-symbolic", "Home", "home"),
        ("instances", "folder-symbolic", "Instances", "instances"),
        ("mods", "package-x-generic-symbolic", "Mods & Plugins", "mods"),
        ("java", "emblem-system-symbolic", "Java", "java"),
        ("accounts", "system-users-symbolic", "Accounts", "accounts"),
        ("settings", "preferences-system-symbolic", "Settings", "settings"),
    ];

    for (id, icon, label, _page) in &nav_items {
        let row = build_nav_row(icon, label, id);
        nav_list.append(&row);
    }

    // Select first row
    if let Some(first_row) = nav_list.row_at_index(0) {
        nav_list.select_row(Some(&first_row));
    }

    // Tag the list box so we can find it in wire_sidebar_to_stack
    nav_list.set_widget_name("main-nav-list");

    sidebar.append(&nav_list);

    // Bottom: active account display
    let account_box = build_account_mini(state.clone(), window);
    sidebar.append(&account_box);

    sidebar
}

fn build_nav_row(icon: &str, label: &str, page_name: &str) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_widget_name(page_name);
    row.add_css_class("nav-row");

    let hbox = gtk4::Box::new(Orientation::Horizontal, 10);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(10);
    hbox.set_margin_bottom(10);

    let img = Image::from_icon_name(icon);
    img.set_pixel_size(18);
    img.add_css_class("nav-icon");

    let lbl = Label::new(Some(label));
    lbl.add_css_class("nav-label");
    lbl.set_halign(gtk4::Align::Start);
    lbl.set_hexpand(true);

    hbox.append(&img);
    hbox.append(&lbl);
    row.set_child(Some(&hbox));
    row
}

fn build_account_mini(
    state: Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) -> gtk4::Box {
    let vbox = gtk4::Box::new(Orientation::Vertical, 0);
    vbox.add_css_class("sidebar-account");

    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_start(12);
    sep.set_margin_end(12);
    vbox.append(&sep);

    let hbox = gtk4::Box::new(Orientation::Horizontal, 10);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(12);
    hbox.set_margin_bottom(12);

    let avatar = libadwaita::Avatar::new(32, None, true);
    avatar.add_css_class("account-avatar");

    let info_box = gtk4::Box::new(Orientation::Vertical, 2);
    info_box.set_hexpand(true);

    let username_label = Label::new(Some("No account"));
    username_label.set_widget_name("sidebar-username");
    username_label.add_css_class("account-name");
    username_label.set_halign(gtk4::Align::Start);
    username_label.set_ellipsize(pango::EllipsizeMode::End);
    username_label.set_max_width_chars(14);

    let type_label = Label::new(Some("Add account →"));
    type_label.set_widget_name("sidebar-account-type");
    type_label.add_css_class("account-type");
    type_label.set_halign(gtk4::Align::Start);

    info_box.append(&username_label);
    info_box.append(&type_label);

    hbox.append(&avatar);
    hbox.append(&info_box);
    vbox.append(&hbox);

    // Update with active account
    {
        let st = state.borrow();
        let accounts = st.accounts.lock().unwrap();
        if let Some(acct) = accounts.active_account() {
            username_label.set_text(&acct.username);
            type_label.set_text(acct.display_type());
            avatar.set_text(Some(&acct.username));
        }
    }

    vbox
}

fn wire_sidebar_to_stack(sidebar: &gtk4::Box, stack: &Stack) {
    // Walk sidebar children to find ListBox
    let mut child = sidebar.first_child();
    while let Some(widget) = child {
        if widget.widget_name() == "main-nav-list" {
            if let Some(list_box) = widget.downcast_ref::<gtk4::ListBox>() {
                let stack_clone = stack.clone();
                list_box.connect_row_selected(move |_, row| {
                    if let Some(row) = row {
                        let page_name = row.widget_name().to_string();
                        stack_clone.set_visible_child_name(&page_name);
                    }
                });
            }
            break;
        }
        child = widget.next_sibling();
    }
}
