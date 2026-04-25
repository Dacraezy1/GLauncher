use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, ScrolledWindow, Image, Entry};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use crate::auth::accounts::Account;
use libadwaita::ApplicationWindow;

pub fn build(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Widget {
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.add_css_class("page");

    // Header
    let header_box = Box::new(Orientation::Horizontal, 8);
    header_box.set_margin_start(24);
    header_box.set_margin_end(24);
    header_box.set_margin_top(20);
    header_box.set_margin_bottom(0);

    let title_lbl = Label::new(Some("Accounts"));
    title_lbl.add_css_class("title-1");
    title_lbl.set_hexpand(true);
    title_lbl.set_halign(gtk4::Align::Start);
    header_box.append(&title_lbl);
    vbox.append(&header_box);

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

    // ── Microsoft Login ────────────────────────────────────────────────
    let ms_group = libadwaita::PreferencesGroup::new();
    ms_group.set_title("Microsoft Account");
    ms_group.set_description(Some("Play online with your Minecraft Java Edition licence."));

    let ms_row = libadwaita::ActionRow::new();
    ms_row.set_title("Login with Microsoft");
    ms_row.set_subtitle("Opens browser for secure login — no password stored in the launcher");

    let ms_icon = Image::from_icon_name("system-users-symbolic");
    ms_row.add_prefix(&ms_icon);

    let ms_btn = Button::with_label("Login");
    ms_btn.add_css_class("suggested-action");
    ms_btn.add_css_class("pill");
    ms_btn.set_valign(gtk4::Align::Center);

    let state_c = state.clone();
    let window_c = window.clone();
    ms_btn.connect_clicked(move |_| {
        crate::ui::dialogs::microsoft_login::show(state_c.clone(), &window_c);
    });

    ms_row.add_suffix(&ms_btn);
    ms_group.add(&ms_row);
    inner.append(&ms_group);

    // ── Offline Account ────────────────────────────────────────────────
    let offline_group = libadwaita::PreferencesGroup::new();
    offline_group.set_title("Offline Account");
    offline_group.set_description(Some("Play offline (singleplayer or servers without online-mode)."));

    let offline_row = libadwaita::ExpanderRow::new();
    offline_row.set_title("Add Offline Account");
    offline_row.set_subtitle("Enter a username to play offline");

    let input_box = Box::new(Orientation::Horizontal, 8);
    input_box.set_margin_start(12);
    input_box.set_margin_end(12);
    input_box.set_margin_top(8);
    input_box.set_margin_bottom(8);

    let username_entry = Entry::new();
    username_entry.set_placeholder_text(Some("Username (3–16 characters)"));
    username_entry.set_hexpand(true);
    username_entry.set_max_length(16);

    let add_btn = Button::with_label("Add");
    add_btn.add_css_class("suggested-action");
    add_btn.set_valign(gtk4::Align::Center);

    input_box.append(&username_entry);
    input_box.append(&add_btn);
    offline_row.add_row(&input_box);

    let state_c = state.clone();
    let window_c = window.clone();
    let username_entry_c = username_entry.clone();
    let offline_row_c = offline_row.clone();
    add_btn.connect_clicked(move |_| {
        let username = username_entry_c.text().to_string();
        let username = username.trim().to_string();
        if username.len() < 3 {
            show_error_dialog(&window_c, "Invalid Username", "Username must be at least 3 characters.");
            return;
        }

        let account = Account::new_offline(&username);
        {
            let st = state_c.borrow_mut();
            let mut accounts = st.accounts.lock().unwrap();
            accounts.add_account(account);
            let _ = accounts.save();
        }

        username_entry_c.set_text("");
        offline_row_c.set_expanded(false);

        // Refresh accounts list (simple: show toast)
        log::info!("Added offline account: {username}");
    });

    offline_group.add(&offline_row);
    inner.append(&offline_group);

    // ── Current Accounts ───────────────────────────────────────────────
    let accounts_group = libadwaita::PreferencesGroup::new();
    accounts_group.set_title("Saved Accounts");
    accounts_group.set_widget_name("accounts-group");

    populate_accounts_group(&accounts_group, &state, window);

    inner.append(&accounts_group);

    scroll.set_child(Some(&inner));
    vbox.append(&scroll);
    vbox.upcast::<gtk4::Widget>()
}

fn populate_accounts_group(
    group: &libadwaita::PreferencesGroup,
    state: &Rc<RefCell<AppState>>,
    window: &ApplicationWindow,
) {
    let st = state.borrow();
    let accounts = st.accounts.lock().unwrap();

    if accounts.accounts.is_empty() {
        let row = libadwaita::ActionRow::new();
        row.set_title("No accounts added yet");
        row.set_subtitle("Add a Microsoft or Offline account above");
        group.add(&row);
        return;
    }

    for account in &accounts.accounts {
        let is_active = accounts.active_account_id.as_deref() == Some(&account.id);

        let row = libadwaita::ActionRow::new();
        row.set_title(&account.username);
        row.set_subtitle(account.display_type());

        let avatar = libadwaita::Avatar::new(36, Some(&account.username), true);
        row.add_prefix(&avatar);

        let btn_box = Box::new(Orientation::Horizontal, 4);
        btn_box.set_valign(gtk4::Align::Center);

        if is_active {
            let active_lbl = Label::new(Some("Active"));
            active_lbl.add_css_class("tag");
            active_lbl.add_css_class("success");
            active_lbl.set_valign(gtk4::Align::Center);
            btn_box.append(&active_lbl);
        } else {
            let select_btn = Button::with_label("Set Active");
            select_btn.add_css_class("pill");
            select_btn.set_valign(gtk4::Align::Center);

            let acct_id = account.id.clone();
            let state_c = state.clone();
            select_btn.connect_clicked(move |_| {
                let st = state_c.borrow_mut();
                let mut accts = st.accounts.lock().unwrap();
                accts.set_active(&acct_id);
                let _ = accts.save();
            });
            btn_box.append(&select_btn);
        }

        let remove_btn = Button::new();
        remove_btn.set_icon_name("edit-delete-symbolic");
        remove_btn.add_css_class("circular");
        remove_btn.add_css_class("destructive-action");
        remove_btn.set_valign(gtk4::Align::Center);
        remove_btn.set_tooltip_text(Some("Remove account"));

        let acct_id = account.id.clone();
        let state_c = state.clone();
        let window_c = window.clone();
        remove_btn.connect_clicked(move |_| {
            let dialog = libadwaita::MessageDialog::new(
                Some(&window_c),
                Some("Remove Account?"),
                Some("This will remove the account from GLauncher."),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("remove", "Remove");
            dialog.set_response_appearance("remove", libadwaita::ResponseAppearance::Destructive);
            let acct_id = acct_id.clone();
            let state_c = state_c.clone();
            dialog.connect_response(None, move |_, resp| {
                if resp == "remove" {
                    let st = state_c.borrow_mut();
                    let mut accts = st.accounts.lock().unwrap();
                    accts.remove_account(&acct_id);
                    let _ = accts.save();
                }
            });
            dialog.present();
        });

        btn_box.append(&remove_btn);
        row.add_suffix(&btn_box);
        group.add(&row);
    }
}

fn show_error_dialog(window: &ApplicationWindow, title: &str, msg: &str) {
    let d = libadwaita::MessageDialog::new(Some(window), Some(title), Some(msg));
    d.add_response("ok", "OK");
    d.present();
}
