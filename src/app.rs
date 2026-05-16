use gpui::*;

use crate::config::types::{AppConfig, UserCommand};
use crate::discovery::types::AppEntry;
use crate::search::fuzzy::fuzzy_match;

const SEARCH_BAR_HEIGHT: f32 = 44.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    App,
    Command,
}

pub struct BattoApp {
    query: String,
    mode: Mode,
    selected_index: usize,
    filtered_apps: Vec<AppEntry>,
    filtered_commands: Vec<UserCommand>,
    all_apps: Vec<AppEntry>,
    all_commands: Vec<UserCommand>,
    focus_handle: FocusHandle,
    config: AppConfig,
}

impl BattoApp {
    pub fn new(
        config: AppConfig,
        all_apps: Vec<AppEntry>,
        all_commands: Vec<UserCommand>,
        focus_handle: FocusHandle,
    ) -> Self {
        let filtered_apps = all_apps.clone();
        Self {
            query: String::new(),
            mode: Mode::App,
            selected_index: 0,
            filtered_apps,
            filtered_commands: all_commands.clone(),
            all_apps,
            all_commands,
            focus_handle,
            config,
        }
    }

    fn update_results(&mut self) {
        match self.mode {
            Mode::App => {
                self.filtered_apps = fuzzy_match(&self.all_apps, &self.query)
                    .into_iter()
                    .cloned()
                    .collect();
            }
            Mode::Command => {
                let search = self.search_term();
                let search_lower = search.to_lowercase();
                self.filtered_commands = if search_lower.is_empty() {
                    self.all_commands.clone()
                } else {
                    self.all_commands
                        .iter()
                        .filter(|c| c.name.to_lowercase().contains(&search_lower))
                        .cloned()
                        .collect()
                };
            }
        }
        self.selected_index = 0;
    }

    fn search_term(&self) -> &str {
        if self.mode == Mode::Command && self.query.starts_with('/') {
            &self.query[1..]
        } else {
            &self.query
        }
    }

    fn launch_selected(&self, cx: &mut Context<Self>) {
        match self.mode {
            Mode::App => {
                if let Some(entry) = self.filtered_apps.get(self.selected_index) {
                    crate::commands::app_launch::launch_app(entry);
                    notify_daemon_launch(&entry.name);
                    cx.quit();
                }
            }
            Mode::Command => {
                if let Some(cmd) = self.filtered_commands.get(self.selected_index) {
                    let args = self.command_args(cmd);
                    let exec = cmd.exec.replace("{{args}}", args);
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&exec)
                        .spawn();
                    cx.quit();
                }
            }
        }
    }

    fn command_args(&self, cmd: &UserCommand) -> &str {
        let search = self.search_term();
        if let Some(rest) = search.strip_prefix(&cmd.name) {
            rest.trim()
        } else {
            ""
        }
    }

    fn grid_columns(&self) -> usize {
        let icon_size = self.config.window.icon_size as f32;
        let cell_width = icon_size + 24.0; // icon + padding
        let window_width = self.config.window.width as f32 - 32.0; // minus horizontal padding
        ((window_width / cell_width).floor() as usize).max(1)
    }

    fn move_up(&mut self) {
        if self.mode == Mode::App {
            let cols = self.grid_columns();
            if self.selected_index >= cols {
                self.selected_index -= cols;
            }
        } else if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_down(&mut self) {
        let max = match self.mode {
            Mode::App => self.filtered_apps.len(),
            Mode::Command => self.filtered_commands.len(),
        };
        if self.mode == Mode::App {
            let cols = self.grid_columns();
            if self.selected_index + cols < max {
                self.selected_index += cols;
            }
        } else if self.selected_index + 1 < max {
            self.selected_index += 1;
        }
    }

    fn move_left(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_right(&mut self) {
        let max = match self.mode {
            Mode::App => self.filtered_apps.len(),
            Mode::Command => self.filtered_commands.len(),
        };
        if self.selected_index + 1 < max {
            self.selected_index += 1;
        }
    }
}

fn notify_daemon_launch(name: &str) {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    if let Ok(mut stream) = UnixStream::connect(crate::daemon::socket_path()) {
        let _ = stream.write_all(format!("launch:{name}").as_bytes());
    }
}

impl Focusable for BattoApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BattoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_size = self.config.window.icon_size as f32;
        let show_name = self.config.window.show_name;
        let mode = self.mode;
        let query = self.query.clone();
        let selected = self.selected_index;
        let filtered_apps = &self.filtered_apps;
        let filtered_commands = &self.filtered_commands;

        div()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(
                |this, event: &KeyDownEvent, _window, cx| {
                    let key = &event.keystroke.key;
                    match key.as_str() {
                        "escape" => cx.quit(),
                        "enter" => this.launch_selected(cx),
                        "up" => {
                            this.move_up();
                            cx.notify();
                        }
                        "down" => {
                            this.move_down();
                            cx.notify();
                        }
                        "left" => {
                            this.move_left();
                            cx.notify();
                        }
                        "right" => {
                            this.move_right();
                            cx.notify();
                        }
                        "backspace" => {
                            this.query.pop();
                            if !this.query.starts_with('/') && this.mode == Mode::Command {
                                this.mode = Mode::App;
                            }
                            this.update_results();
                            cx.notify();
                        }
                        _ => {
                            if let Some(ch) = event.keystroke.key_char.as_ref() {
                                if ch == "/"
                                    && this.query.is_empty()
                                    && !this.all_commands.is_empty()
                                {
                                    this.mode = Mode::Command;
                                    this.query.push('/');
                                    this.update_results();
                                    cx.notify();
                                } else if ch.chars().all(|c| !c.is_control()) {
                                    this.query.push_str(ch);
                                    this.update_results();
                                    cx.notify();
                                }
                            }
                        }
                    }
                },
            ))
            .child(render_search_bar(&query, mode))
            .child(match mode {
                Mode::App => {
                    render_app_grid(filtered_apps, selected, icon_size, show_name)
                        .into_any_element()
                }
                Mode::Command => {
                    render_command_list(filtered_commands, selected).into_any_element()
                }
            })
    }
}

fn render_search_bar(query: &str, mode: Mode) -> Div {
    div()
        .w_full()
        .h(px(SEARCH_BAR_HEIGHT))
        .flex_shrink_0()
        .px(px(16.0))
        .border_b_1()
        .border_color(rgb(0x313244))
        .flex()
        .items_center()
        .child(if query.is_empty() {
            div()
                .child("Search...")
                .text_color(rgb(0x585b70))
                .text_size(px(16.0))
        } else if mode == Mode::Command {
            div()
                .child(query[1..].to_string())
                .text_color(rgb(0xcdd6f4))
                .text_size(px(16.0))
        } else {
            div()
                .child(query.to_string())
                .text_color(rgb(0xcdd6f4))
                .text_size(px(16.0))
        })
}

fn render_app_grid(
    apps: &[AppEntry],
    selected: usize,
    icon_size: f32,
    show_name: bool,
) -> impl IntoElement {
    div()
        .id("app-grid")
        .flex_1()
        .w_full()
        .overflow_y_scroll()
        .px(px(16.0))
        .py(px(12.0))
        .children(apps.iter().enumerate().map(|(i, entry)| {
            let is_selected = i == selected;
            let mut cell = div()
                .w_full()
                .h(px(icon_size + 12.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .rounded(px(6.0));

            if is_selected {
                cell = cell.bg(rgb(0x313244));
            }

            let icon = if let Some(ref path) = entry.icon_path {
                img(std::path::PathBuf::from(path.clone()))
                    .size(px(icon_size))
                    .rounded(px(4.0))
                    .into_any_element()
            } else {
                div()
                    .size(px(icon_size))
                    .rounded(px(6.0))
                    .bg(rgb(0x45475a))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(entry.name.chars().next().unwrap_or('?').to_string())
                    .text_color(rgb(0xcdd6f4))
                    .text_size(px(icon_size * 0.4))
                    .into_any_element()
            };

            cell = cell.child(icon);

            if show_name {
                cell = cell.child(
                    div()
                        .child(entry.name.clone())
                        .text_color(if is_selected {
                            rgb(0xcdd6f4)
                        } else {
                            rgb(0xa6adc8)
                        })
                        .text_size(px(13.0))
                        .overflow_hidden(),
                );
            }

            cell
        }))
}

fn render_command_list(commands: &[UserCommand], selected: usize) -> impl IntoElement {
    div()
        .id("command-list")
        .flex_1()
        .w_full()
        .overflow_y_scroll()
        .children(commands.iter().enumerate().map(|(i, cmd)| {
            let is_selected = i == selected;
            let el = div()
                .w_full()
                .px(px(16.0))
                .py(px(8.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .child(format!("/{}", cmd.name))
                        .text_color(rgb(0x89b4fa))
                        .text_size(px(14.0)),
                )
                .child(
                    div()
                        .child(cmd.description.clone())
                        .text_color(if is_selected {
                            rgb(0xcdd6f4)
                        } else {
                            rgb(0xa6adc8)
                        })
                        .text_size(px(13.0)),
                );
            if is_selected {
                el.bg(rgb(0x313244))
            } else {
                el
            }
        }))
}
