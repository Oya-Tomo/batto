use batto::{app::BattoApp, config, daemon, discovery};
use gpui::*;

fn main() {
    if !daemon::is_daemon_running() {
        daemon::start_daemon();
    }

    let (cfg, all_apps, commands) = match daemon::request_all() {
        Ok(data) => (data.config, data.apps, data.commands),
        Err(_) => (
            config::load_config(),
            discovery::discover_apps(),
            Vec::new(),
        ),
    };

    Application::new().run(move |cx: &mut App| {
        let width = cfg.window.width as f32;
        let height = cfg.window.list_height as f32 + 92.0;

        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(width), px(height)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("batto".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_decorations: Some(WindowDecorations::Client),
                focus: true,
                show: true,
                ..Default::default()
            },
            |_window, cx| {
                let focus_handle = cx.focus_handle();
                cx.new(|cx| {
                    cx.focus_self(_window);
                    BattoApp::new(cfg.clone(), all_apps.clone(), commands.clone(), focus_handle, _window, cx)
                })
            },
        )
        .expect("failed to open window");

        cx.activate(true);
    });
}
