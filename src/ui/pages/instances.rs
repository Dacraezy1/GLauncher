use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, ScrolledWindow, Image, Entry};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use crate::ui::pages::home::build_page_header;
use libadwaita::ApplicationWindow;

pub fn build(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Widget {
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.add_css_class("page");

    // Toolbar
    let toolbar_box = Box::new(Orientation::Horizontal, 8);
    toolbar_box.set_margin_start(24);
    toolbar_box.set_margin_end(24);
    toolbar_box.set_margin_top(20);
    toolbar_box.set_margin_bottom(0);

    let title_lbl = Label::new(Some("Instances"));
    title_lbl.add_css_class("title-1");
    title_lbl.set_hexpand(true);
    title_lbl.set_halign(gtk4::Align::Start);

    let new_btn = Button::new();
    new_btn.set_icon_name("list-add-symbolic");
    new_btn.add_css_class("suggested-action");
    new_btn.add_css_class("pill");
    new_btn.set_label("New Instance");
    new_btn.set_icon_name("list-add-symbolic");

    let state_clone = state.clone();
    let window_clone = window.clone();
    new_btn.connect_clicked(move |_| {
        crate::ui::dialogs::new_instance::show(&state_clone, &window_clone);
    });

    toolbar_box.append(&title_lbl);
    toolbar_box.append(&new_btn);
    vbox.append(&toolbar_box);

    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_margin_start(24);
    sep.set_margin_end(24);
    sep.set_margin_top(12);
    sep.set_margin_bottom(4);
    vbox.append(&sep);

    // Search
    let search_entry = Entry::new();
    search_entry.set_placeholder_text(Some("Search instances..."));
    search_entry.set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("system-search-symbolic"));
    search_entry.set_margin_start(24);
    search_entry.set_margin_end(24);
    search_entry.set_margin_top(8);
    search_entry.set_margin_bottom(8);
    vbox.append(&search_entry);

    // List
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let list_box = gtk4::ListBox::new();
    list_box.add_css_class("boxed-list");
    list_box.set_margin_start(24);
    list_box.set_margin_end(24);
    list_box.set_margin_top(4);
    list_box.set_margin_bottom(24);
    list_box.set_selection_mode(gtk4::SelectionMode::None);
    list_box.set_widget_name("instances-list");

    populate_instances_list(&list_box, &state, window);

    scroll.set_child(Some(&list_box));
    vbox.append(&scroll);

    vbox.into()
}

fn populate_instances_list(
    list_box: &gtk4::ListBox,
    state: &Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) {
    // Clear
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let st = state.borrow();
    let instances = st.instances.lock().unwrap();

    if instances.instances.is_empty() {
        let row = libadwaita::ActionRow::new();
        row.set_title("No instances yet");
        row.set_subtitle("Click \"New Instance\" to create one");
        let icon = Image::from_icon_name("folder-open-symbolic");
        row.add_prefix(&icon);
        list_box.append(&row);
        return;
    }

    for inst in &instances.instances {
        let row = build_instance_row(inst, state, window);
        list_box.append(&row);
    }
}

fn build_instance_row(
    inst: &crate::minecraft::instances::Instance,
    state: &Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) -> gtk4::Widget {
    let row = libadwaita::ActionRow::new();
    row.set_title(&inst.name);
    row.set_subtitle(&format!(
        "{} • {} • {:.0}MB RAM",
        inst.minecraft_version,
        inst.loader_display(),
        inst.jvm_settings.memory_max_mb
    ));
    row.set_activatable(false);

    // Icon
    let icon = Image::from_icon_name("applications-games-symbolic");
    icon.set_pixel_size(40);
    row.add_prefix(&icon);

    // Buttons
    let btn_box = Box::new(Orientation::Horizontal, 4);
    btn_box.set_valign(gtk4::Align::Center);

    let play_btn = Button::new();
    play_btn.set_icon_name("media-playback-start-symbolic");
    play_btn.add_css_class("suggested-action");
    play_btn.add_css_class("circular");
    play_btn.set_tooltip_text(Some("Launch"));

    let inst_id = inst.id.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    play_btn.connect_clicked(move |_| {
        crate::ui::dialogs::launch::launch_instance(&inst_id, state_clone.clone(), &window_clone);
    });

    let folder_btn = Button::new();
    folder_btn.set_icon_name("folder-open-symbolic");
    folder_btn.add_css_class("circular");
    folder_btn.set_tooltip_text(Some("Open folder"));

    let inst_dir = inst.minecraft_dir().to_string_lossy().to_string();
    folder_btn.connect_clicked(move |_| {
        let _ = open::that(&inst_dir);
    });

    let edit_btn = Button::new();
    edit_btn.set_icon_name("document-edit-symbolic");
    edit_btn.add_css_class("circular");
    edit_btn.set_tooltip_text(Some("Edit settings"));

    let inst_id2 = inst.id.clone();
    let state_clone2 = state.clone();
    let window_clone2 = window.clone();
    edit_btn.connect_clicked(move |_| {
        crate::ui::dialogs::instance_settings::show(&inst_id2, state_clone2.clone(), &window_clone2);
    });

    let delete_btn = Button::new();
    delete_btn.set_icon_name("edit-delete-symbolic");
    delete_btn.add_css_class("circular");
    delete_btn.add_css_class("destructive-action");
    delete_btn.set_tooltip_text(Some("Delete instance"));

    let inst_id3 = inst.id.clone();
    let state_clone3 = state.clone();
    let window_clone3 = window.clone();
    let row_widget = row.clone();
    delete_btn.connect_clicked(move |_| {
        let dialog = libadwaita::MessageDialog::new(
            Some(&window_clone3),
            Some("Delete Instance?"),
            Some("This will permanently delete the instance and all its files."),
        );
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("delete", "Delete");
        dialog.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);

        let inst_id = inst_id3.clone();
        let state = state_clone3.clone();
        let row = row_widget.clone();
        dialog.connect_response(None, move |_, response| {
            if response == "delete" {
                let mut st = state.borrow_mut();
                let mut instances = st.instances.lock().unwrap();
                let _ = instances.remove(&inst_id);
                // Remove row from parent
                if let Some(parent) = row.parent() {
                    if let Some(list) = parent.downcast_ref::<gtk4::ListBox>() {
                        list.remove(&row);
                    }
                }
            }
        });
        dialog.present();
    });

    btn_box.append(&folder_btn);
    btn_box.append(&edit_btn);
    btn_box.append(&play_btn);
    btn_box.append(&delete_btn);
    row.add_suffix(&btn_box);

    row.into()
}
