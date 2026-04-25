use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{Box, Orientation, Label, Button, Entry};
use std::rc::Rc;
use std::cell::RefCell;
use crate::ui::state::AppState;
use libadwaita::ApplicationWindow;

pub fn show(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) {
    let (auth_url, pkce_verifier) = crate::auth::microsoft::get_auth_url();

    let dialog = libadwaita::Window::new();
    dialog.set_title(Some("Microsoft Login"));
    dialog.set_transient_for(Some(window));
    dialog.set_modal(true);
    dialog.set_default_size(500, 400);
    dialog.set_resizable(false);

    let content = Box::new(Orientation::Vertical, 0);

    let header = libadwaita::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("flat");
    header.pack_start(&cancel_btn);
    content.append(&header);

    let inner = Box::new(Orientation::Vertical, 16);
    inner.set_margin_start(24);
    inner.set_margin_end(24);
    inner.set_margin_top(20);
    inner.set_margin_bottom(20);

    // Step 1: open browser
    let step1_lbl = Label::new(Some("Step 1 — Open Browser"));
    step1_lbl.add_css_class("title-3");
    step1_lbl.set_halign(gtk4::Align::Start);
    inner.append(&step1_lbl);

    let desc = Label::new(Some(
        "Click the button below to open the Microsoft login page in your browser.\n\
         Sign in with your Microsoft account, then paste the redirect URL back here."
    ));
    desc.set_wrap(true);
    desc.add_css_class("body");
    desc.set_halign(gtk4::Align::Start);
    inner.append(&desc);

    let open_btn = Button::with_label("Open Login Page");
    open_btn.add_css_class("suggested-action");
    open_btn.add_css_class("pill");
    let auth_url_clone = auth_url.clone();
    open_btn.connect_clicked(move |_| {
        let _ = open::that(&auth_url_clone);
    });
    inner.append(&open_btn);

    // Show the URL for manual copy
    let url_box = Box::new(Orientation::Horizontal, 8);
    let url_entry = Entry::new();
    url_entry.set_text(&auth_url);
    url_entry.set_editable(false);
    url_entry.set_hexpand(true);
    url_entry.add_css_class("monospace");
    let copy_btn = Button::new();
    copy_btn.set_icon_name("edit-copy-symbolic");
    copy_btn.set_tooltip_text(Some("Copy URL"));
    let auth_url_c2 = auth_url.clone();
    copy_btn.connect_clicked(move |btn| {
        if let Some(display) = gdk4::Display::default() {
            display.clipboard().set_text(&auth_url_c2);
        }
    });
    url_box.append(&url_entry);
    url_box.append(&copy_btn);
    inner.append(&url_box);

    // Separator
    inner.append(&gtk4::Separator::new(Orientation::Horizontal));

    // Step 2: paste redirect URL
    let step2_lbl = Label::new(Some("Step 2 — Paste Redirect URL"));
    step2_lbl.add_css_class("title-3");
    step2_lbl.set_halign(gtk4::Align::Start);
    inner.append(&step2_lbl);

    let desc2 = Label::new(Some(
        "After login, your browser will redirect to a blank page.\n\
         Copy the full URL from the address bar and paste it here."
    ));
    desc2.set_wrap(true);
    desc2.add_css_class("body");
    desc2.set_halign(gtk4::Align::Start);
    inner.append(&desc2);

    let redirect_entry = Entry::new();
    redirect_entry.set_placeholder_text(Some("https://login.live.com/oauth20_desktop.srf?code=..."));
    redirect_entry.add_css_class("monospace");
    inner.append(&redirect_entry);

    let status_lbl = Label::new(Some(""));
    status_lbl.add_css_class("dim-label");
    status_lbl.set_halign(gtk4::Align::Start);
    inner.append(&status_lbl);

    let login_btn = Button::with_label("Log In");
    login_btn.add_css_class("suggested-action");
    inner.append(&login_btn);

    content.append(&inner);
    dialog.set_content(Some(&content));

    // Wire cancel
    let dialog_c = dialog.clone();
    cancel_btn.connect_clicked(move |_| dialog_c.close());

    // Wire login
    let dialog_c = dialog.clone();
    let state_c = state.clone();
    let window_c = window.clone();
    let verifier = pkce_verifier.clone();

    login_btn.connect_clicked(move |btn| {
        let redirect_url = redirect_entry.text().to_string().trim().to_string();
        if redirect_url.is_empty() {
            status_lbl.set_text("Please paste the redirect URL.");
            return;
        }

        // Extract code from redirect URL
        let code = match extract_code_from_redirect(&redirect_url) {
            Some(c) => c,
            None => {
                status_lbl.set_text("Could not find 'code' in URL. Make sure you copied the full URL.");
                return;
            }
        };

        btn.set_sensitive(false);
        btn.set_label("Logging in...");
        status_lbl.set_text("Authenticating with Microsoft...");

        let http_client = state_c.borrow().http_client.clone();
        let verifier = verifier.clone();
        let state_c2 = state_c.clone();
        let dialog_c2 = dialog_c.clone();
        let status_c = status_lbl.clone();
        let btn_c = btn.clone();

        let (tx, rx) = glib::MainContext::channel::<Result<crate::auth::accounts::Account, String>>(
            glib::Priority::DEFAULT,
        );

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = rt.block_on(async {
                crate::auth::microsoft::full_auth_from_code(&http_client, &code, &verifier).await
            });
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });

        rx.attach(None, move |result| {
            match result {
                Ok(account) => {
                    let username = account.username.clone();
                    {
                        let mut st = state_c2.borrow_mut();
                        let mut accounts = st.accounts.lock().unwrap();
                        accounts.add_account(account);
                        let _ = accounts.save();
                    }
                    log::info!("Logged in as {username}");
                    dialog_c2.close();
                }
                Err(e) => {
                    status_c.set_text(&format!("Login failed: {e}"));
                    btn_c.set_sensitive(true);
                    btn_c.set_label("Try Again");
                }
            }
            glib::ControlFlow::Break
        });
    });

    dialog.present();
}

fn extract_code_from_redirect(url: &str) -> Option<String> {
    // Handle both formats:
    // https://login.live.com/oauth20_desktop.srf?code=XXX&...
    // or just the code itself
    if !url.contains('?') {
        // Maybe they pasted just the code
        if url.len() > 10 && !url.contains(' ') {
            return Some(url.to_string());
        }
        return None;
    }

    url.split('?')
        .nth(1)?
        .split('&')
        .find(|p| p.starts_with("code="))
        .map(|p| p.trim_start_matches("code=").to_string())
        .filter(|c| !c.is_empty())
}
