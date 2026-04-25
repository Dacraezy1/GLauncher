mod ui;
mod minecraft;
mod auth;
mod mods;
mod java;
mod utils;

use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita::Application;

const APP_ID: &str = "io.github.Dacraezy1.GLauncher";

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_startup(|_| {
        // Load custom CSS
        let css = gtk4::CssProvider::new();
        css.load_from_data(include_str!("../assets/ui/style.css"));
        gtk4::style_context_add_provider_for_display(
            &gdk4::Display::default().unwrap(),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(ui::window::build_ui);

    app.run();
}
