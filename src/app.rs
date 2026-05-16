use gpui::*;
use std::ops::Range;

use crate::config::types::{AppConfig, CommandArg, QueryHandlerInfo, QueryResult, UserCommand};
use crate::discovery::types::AppEntry;
use crate::search::fuzzy::fuzzy_match;

const SEARCH_BAR_HEIGHT: f32 = 44.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    App,
    Command,
    Calculator,
}

pub struct BattoApp {
    query: String,
    cursor_pos: usize,
    mode: Mode,
    selected_index: usize,
    filtered_apps: Vec<AppEntry>,
    filtered_commands: Vec<UserCommand>,
    all_apps: Vec<AppEntry>,
    all_commands: Vec<UserCommand>,
    query_handlers: Vec<QueryHandlerInfo>,
    query_results: Vec<QueryResult>,
    active_query_prefix: Option<String>,
    focus_handle: FocusHandle,
    config: AppConfig,
    calc_result: Option<String>,
    filling_args: bool,
    active_arg_index: usize,
    arg_values: Vec<String>,
    arg_cursor_pos: usize,
    arg_search: String,
    arg_choice_idx: usize,
}

impl BattoApp {
    pub fn new(
        config: AppConfig,
        all_apps: Vec<AppEntry>,
        all_commands: Vec<UserCommand>,
        query_handlers: Vec<QueryHandlerInfo>,
        focus_handle: FocusHandle,
    ) -> Self {
        let filtered_apps = all_apps.clone();
        Self {
            query: String::new(),
            cursor_pos: 0,
            mode: Mode::App,
            selected_index: 0,
            filtered_apps,
            filtered_commands: all_commands.clone(),
            all_apps,
            all_commands,
            query_handlers,
            query_results: Vec::new(),
            active_query_prefix: None,
            focus_handle,
            config,
            calc_result: None,
            filling_args: false,
            active_arg_index: 0,
            arg_values: Vec::new(),
            arg_cursor_pos: 0,
            arg_search: String::new(),
            arg_choice_idx: 0,
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
                let search = self.search_term().to_string();
                // Check if query matches an on_query handler prefix
                let matched_prefix = self.query_handlers.iter().find(|h| {
                    search.starts_with(&h.prefix) &&
                        (search.len() == h.prefix.len() || search.as_bytes().get(h.prefix.len()) == Some(&b' '))
                });
                if let Some(handler) = matched_prefix {
                    let query_text = if search.len() > handler.prefix.len() {
                        &search[handler.prefix.len() + 1..]
                    } else {
                        ""
                    };
                    let prefix = handler.prefix.clone();
                    self.query_results = crate::daemon::request_query(&prefix, query_text)
                        .unwrap_or_default();
                    self.active_query_prefix = Some(prefix);
                    self.filtered_commands.clear();
                } else {
                    self.active_query_prefix = None;
                    self.query_results.clear();
                    let search_lower = search.to_lowercase();
                    self.filtered_commands = if search_lower.is_empty() {
                        self.all_commands.clone()
                    } else {
                        self.all_commands
                            .iter()
                            .filter(|c| {
                                let name_lower = c.name.to_lowercase();
                                name_lower.starts_with(&search_lower)
                                    || search_lower.starts_with(&name_lower)
                            })
                            .cloned()
                            .collect()
                    };
                    // Check if we should enter arg-filling mode
                    if !self.filling_args {
                        if let Some(cmd) = self.filtered_commands.first() {
                            if cmd.name.to_lowercase() == search_lower && !cmd.args.is_empty() {
                                self.filling_args = true;
                                self.active_arg_index = 0;
                                self.arg_cursor_pos = 0;
                                self.arg_values = cmd.args.iter().map(|a| {
                                    if a.arg_type == "literal" && !a.choices.is_empty() {
                                        a.choices.first().map(|c| c.value.clone()).unwrap_or_default()
                                    } else {
                                        String::new()
                                    }
                                }).collect();
                                self.reset_arg_search();
                            }
                        }
                    }
                }
            }
            Mode::Calculator => {
                let expr = &self.query[1..]; // skip '=' prefix
                if expr.is_empty() {
                    self.calc_result = None;
                } else {
                    self.calc_result = crate::commands::calculator::evaluate(expr).ok().map(|v| {
                        if v == v.floor() && v.abs() < i64::MAX as f64 {
                            format!("{}", v as i64)
                        } else {
                            format!("{v:.6}")
                        }
                    });
                }
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
                if self.active_query_prefix.is_some() {
                    if let Some(result) = self.query_results.get(self.selected_index) {
                        let _ = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(&result.exec)
                            .spawn();
                        cx.quit();
                    }
                } else if self.filling_args {
                    if let Some(cmd) = self.filtered_commands.first() {
                        let exec = self.build_exec(cmd);
                        let _ = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(&exec)
                            .spawn();
                        cx.quit();
                    }
                } else if let Some(cmd) = self.filtered_commands.get(self.selected_index) {
                    let args = self.command_args(cmd);
                    let exec = cmd.exec.replace("{{args}}", args);
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&exec)
                        .spawn();
                    cx.quit();
                }
            }
            Mode::Calculator => {
                // Copy result to clipboard and quit
                if let Some(ref result) = self.calc_result {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(format!("echo -n '{result}' | xclip -selection clipboard 2>/dev/null || echo -n '{result}' | wl-copy 2>/dev/null"))
                        .spawn();
                }
                cx.quit();
            }
        }
    }

    fn reset_arg_search(&mut self) {
        let is_literal = self.filtered_commands.first()
            .and_then(|c| c.args.get(self.active_arg_index))
            .map(|a| a.arg_type == "literal" && !a.choices.is_empty())
            .unwrap_or(false);
        if is_literal {
            let current_val = self.arg_values.get(self.active_arg_index).cloned().unwrap_or_default();
            let display = self.filtered_commands.first()
                .and_then(|c| c.args.get(self.active_arg_index))
                .and_then(|a| a.choices.iter().find(|ch| ch.value == current_val))
                .map(|ch| ch.name.clone())
                .unwrap_or_default();
            self.arg_search = display;
            self.arg_choice_idx = 0;
        } else {
            self.arg_search.clear();
            self.arg_choice_idx = 0;
        }
    }

    fn filtered_choices(&self) -> Vec<(String, String)> {
        let arg = self.filtered_commands.first()
            .and_then(|c| c.args.get(self.active_arg_index));
        match arg {
            Some(a) if a.arg_type == "literal" && !a.choices.is_empty() => {
                let search = self.arg_search.to_lowercase();
                a.choices.iter()
                    .filter(|c| search.is_empty() || c.name.to_lowercase().contains(&search))
                    .map(|c| (c.name.clone(), c.value.clone()))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn commit_literal_choice(&mut self) {
        let choices = self.filtered_choices();
        if let Some((_, value)) = choices.get(self.arg_choice_idx).cloned() {
            if let Some(v) = self.arg_values.get_mut(self.active_arg_index) {
                *v = value;
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

    fn build_exec(&self, cmd: &UserCommand) -> String {
        let mut exec = cmd.exec.clone();
        // Replace {{args}} with all arg values joined
        let args = self.arg_values.join(" ");
        exec = exec.replace("{{args}}", &args);
        // Replace named args like {{arg_name}}
        for (i, arg) in cmd.args.iter().enumerate() {
            let value = self.arg_values.get(i).map(|s| s.as_str()).unwrap_or("");
            exec = exec.replace(&format!("{{{{{}}}}}", arg.name), value);
        }
        exec
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
            Mode::Command => {
                if self.active_query_prefix.is_some() {
                    self.query_results.len()
                } else {
                    self.filtered_commands.len()
                }
            }
            Mode::Calculator => return,
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_size = self.config.window.icon_size as f32;
        let show_name = self.config.window.show_name;
        let mode = self.mode;
        let query = self.query.clone();
        let cursor_pos = self.cursor_pos;
        let selected = self.selected_index;
        let filtered_apps = &self.filtered_apps;
        let filtered_commands = &self.filtered_commands;
        let calc_result = self.calc_result.clone();
        let active_query_prefix = self.active_query_prefix.clone();
        let query_results = &self.query_results;
        let filling_args = self.filling_args;
        let active_arg_index = self.active_arg_index;
        let arg_cursor_pos = self.arg_cursor_pos;
        let arg_values = self.arg_values.clone();
        let arg_search = self.arg_search.clone();
        let arg_choice_idx = self.arg_choice_idx;
        let filtered_choices = self.filtered_choices();
        let current_cmd_args: Vec<CommandArg> = if filling_args {
            self.filtered_commands.first()
                .map(|c| c.args.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let focus_handle = self.focus_handle.clone();
        let entity = cx.entity().clone();

        let content = div()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(
                |this, event: &KeyDownEvent, _window, cx| {
                    let key = &event.keystroke.key;
                    // When filling args, redirect most keys to arg input
                    if this.filling_args {
                        let is_literal = this.filtered_commands.first()
                            .and_then(|c| c.args.get(this.active_arg_index))
                            .map(|a| a.arg_type == "literal" && !a.choices.is_empty())
                            .unwrap_or(false);
                        let cmd_args = this.filtered_commands.first()
                            .map(|c| c.args.len())
                            .unwrap_or(0);
                        match key.as_str() {
                            "escape" => {
                                this.filling_args = false;
                            }
                            "enter" => {
                                if is_literal {
                                    this.commit_literal_choice();
                                }
                                if this.active_arg_index + 1 < cmd_args {
                                    this.active_arg_index += 1;
                                    this.arg_cursor_pos = 0;
                                    this.reset_arg_search();
                                } else {
                                    this.launch_selected(cx);
                                }
                            }
                            "tab" => {
                                if is_literal {
                                    this.commit_literal_choice();
                                }
                                if cmd_args > 0 {
                                    this.active_arg_index = (this.active_arg_index + 1) % cmd_args;
                                    this.arg_cursor_pos = this.arg_values.get(this.active_arg_index).map(|s| s.len()).unwrap_or(0);
                                    this.reset_arg_search();
                                }
                            }
                            "up" if is_literal => {
                                if this.arg_choice_idx > 0 {
                                    this.arg_choice_idx -= 1;
                                }
                            }
                            "down" if is_literal => {
                                let count = this.filtered_choices().len();
                                if this.arg_choice_idx + 1 < count {
                                    this.arg_choice_idx += 1;
                                }
                            }
                            "backspace" if is_literal => {
                                if !this.arg_search.is_empty() {
                                    this.arg_search.pop();
                                    this.arg_choice_idx = 0;
                                } else if this.active_arg_index > 0 {
                                    this.active_arg_index -= 1;
                                    this.arg_cursor_pos = this.arg_values.get(this.active_arg_index).map(|s| s.len()).unwrap_or(0);
                                    this.reset_arg_search();
                                }
                            }
                            "backspace" if !is_literal => {
                                if this.arg_cursor_pos > 0 {
                                    if let Some(val) = this.arg_values.get_mut(this.active_arg_index) {
                                        val.remove(this.arg_cursor_pos - 1);
                                        this.arg_cursor_pos -= 1;
                                    }
                                } else if this.active_arg_index > 0 {
                                    this.active_arg_index -= 1;
                                    this.arg_cursor_pos = this.arg_values.get(this.active_arg_index).map(|s| s.len()).unwrap_or(0);
                                    this.reset_arg_search();
                                }
                            }
                            "left" if !is_literal => {
                                if this.arg_cursor_pos > 0 {
                                    this.arg_cursor_pos -= 1;
                                }
                            }
                            "right" if !is_literal => {
                                let len = this.arg_values.get(this.active_arg_index).map(|s| s.len()).unwrap_or(0);
                                if this.arg_cursor_pos < len {
                                    this.arg_cursor_pos += 1;
                                }
                            }
                            _ => {}
                        }
                        cx.notify();
                        return;
                    }
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
                            if this.cursor_pos > 0 {
                                this.cursor_pos -= 1;
                                cx.notify();
                            }
                        }
                        "right" => {
                            if this.cursor_pos < this.query.len() {
                                this.cursor_pos += 1;
                                cx.notify();
                            }
                        }
                        "backspace" => {
                            if this.cursor_pos > 0 {
                                this.query.remove(this.cursor_pos - 1);
                                this.cursor_pos -= 1;
                            }
                            if this.query.is_empty() {
                                if this.mode == Mode::Command || this.mode == Mode::Calculator {
                                    this.mode = Mode::App;
                                }
                            }
                            this.update_results();
                            cx.notify();
                        }
                        "tab" => {
                            let search = this.search_term().to_string();
                            let prefix_len = this.query.len() - search.len();
                            let completion = match this.mode {
                                Mode::App => {
                                    this.filtered_apps.first().map(|a| a.name.clone())
                                }
                                Mode::Command => {
                                    if this.active_query_prefix.is_none() {
                                        this.filtered_commands.first().map(|c| c.name.clone())
                                            .or_else(|| {
                                                this.query_handlers.iter()
                                                    .find(|h| h.prefix.starts_with(&search))
                                                    .map(|h| h.prefix.clone())
                                            })
                                    } else {
                                        None
                                    }
                                }
                                Mode::Calculator => None,
                            };
                            if let Some(name) = completion {
                                if name.len() > search.len() {
                                    this.query = format!("{}{}", &this.query[..prefix_len], name);
                                    this.cursor_pos = this.query.len();
                                    this.update_results();
                                    cx.notify();
                                }
                            }
                        }
                        "delete" => {
                            if this.cursor_pos < this.query.len() {
                                this.query.remove(this.cursor_pos);
                                this.update_results();
                                cx.notify();
                            }
                        }
                        "home" => {
                            let min = if this.mode == Mode::Command || this.mode == Mode::Calculator { 1 } else { 0 };
                            this.cursor_pos = min;
                            cx.notify();
                        }
                        "end" => {
                            this.cursor_pos = this.query.len();
                            cx.notify();
                        }
                        _ => {}
                    }
                },
            ))
            .child(render_search_bar(&query, mode, cursor_pos))
            .child(match mode {
                Mode::App => {
                    render_app_grid(filtered_apps, selected, icon_size, show_name)
                        .into_any_element()
                }
                Mode::Command => {
                    if active_query_prefix.is_some() {
                        render_query_results(query_results, selected).into_any_element()
                    } else if filling_args {
                        render_arg_form(&current_cmd_args, &arg_values, active_arg_index, arg_cursor_pos, &arg_search, arg_choice_idx, &filtered_choices).into_any_element()
                    } else {
                        render_command_list(filtered_commands, selected).into_any_element()
                    }
                }
                Mode::Calculator => {
                    render_calculator(&query, &calc_result).into_any_element()
                }
            });

        InputHandlerWrapper {
            child: content.into_any_element(),
            focus_handle,
            entity,
        }
    }
}

impl EntityInputHandler for BattoApp {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        Some(self.query[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let utf16_cursor = self.query[..self.cursor_pos].encode_utf16().count();
        Some(UTF16Selection {
            range: utf16_cursor..utf16_cursor,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.filling_args {
            let is_literal = self.filtered_commands.first()
                .and_then(|c| c.args.get(self.active_arg_index))
                .map(|a| a.arg_type == "literal" && !a.choices.is_empty())
                .unwrap_or(false);
            if is_literal {
                self.arg_search.push_str(text);
                self.arg_choice_idx = 0;
            } else if let Some(val) = self.arg_values.get_mut(self.active_arg_index) {
                val.insert_str(self.arg_cursor_pos, text);
                self.arg_cursor_pos += text.len();
            }
            cx.notify();
            return;
        }

        if text.starts_with('/') && self.query.is_empty() && !self.all_commands.is_empty() {
            self.mode = Mode::Command;
        } else if text.starts_with('=') && self.query.is_empty() {
            self.mode = Mode::Calculator;
        }
        self.query.insert_str(self.cursor_pos, text);
        self.cursor_pos += text.len();
        self.update_results();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        _new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

fn render_search_bar(query: &str, mode: Mode, cursor_pos: usize) -> Div {
    if query.is_empty() {
        return div()
            .w_full()
            .h(px(SEARCH_BAR_HEIGHT))
            .flex_shrink_0()
            .px(px(16.0))
            .border_b_1()
            .border_color(rgb(0x313244))
            .flex()
            .items_center()
            .child(div().child("Search...").text_color(rgb(0x585b70)).text_size(px(16.0)))
            .child(div().w(px(1.5)).h(px(18.0)).bg(rgb(0xcdd6f4)));
    }

    let text_color = rgb(0xcdd6f4);
    let prefix_color = match mode {
        Mode::Command => Some(rgb(0x89b4fa)),  // blue for /
        Mode::Calculator => Some(rgb(0xa6e3a1)), // green for =
        Mode::App => None,
    };

    // Split: [prefix char] [before cursor (rest)] |cursor| [after cursor]
    let mut bar = div()
        .w_full()
        .h(px(SEARCH_BAR_HEIGHT))
        .flex_shrink_0()
        .px(px(16.0))
        .border_b_1()
        .border_color(rgb(0x313244))
        .flex()
        .items_center();

    // Highlighted prefix character
    if let Some(color) = prefix_color {
        bar = bar.child(div().child(query[..1].to_string()).text_color(color).text_size(px(16.0)));
    }

    let start = if prefix_color.is_some() { 1 } else { 0 };
    let display = &query[start..];
    let cpos = cursor_pos.saturating_sub(start);

    let before = display[..cpos].to_string();
    let after = display[cpos..].to_string();

    let input = div()
        .flex()
        .items_center()
        .child(div().child(before).text_color(text_color).text_size(px(16.0)))
        .child(div().w(px(1.5)).h(px(18.0)).bg(rgb(0xcdd6f4)))
        .child(div().child(after).text_color(text_color).text_size(px(16.0)));
    bar = bar.child(input);

    bar
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

fn render_calculator(query: &str, result: &Option<String>) -> impl IntoElement {
    let expr = &query[1..]; // skip '='
    let result_text = match result {
        Some(v) => v.clone(),
        None if expr.is_empty() => "Type an expression...".to_string(),
        None => "Invalid expression".to_string(),
    };
    let has_result = result.is_some();
    div()
        .id("calculator")
        .flex_1()
        .w_full()
        .px(px(16.0))
        .py(px(16.0))
        .child(
            div()
                .w_full()
                .px(px(16.0))
                .py(px(12.0))
                .rounded(px(8.0))
                .bg(rgb(0x313244))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .child(if expr.is_empty() { " ".to_string() } else { expr.to_string() })
                        .text_color(rgb(0xa6adc8))
                        .text_size(px(14.0)),
                )
                .child(
                    div()
                        .child(result_text)
                        .text_color(if has_result {
                            rgb(0xa6e3a1)
                        } else {
                            rgb(0xf38ba8)
                        })
                        .text_size(px(20.0)),
                ),
        )
}

fn render_query_results(results: &[QueryResult], selected: usize) -> impl IntoElement {
    div()
        .id("query-results")
        .flex_1()
        .w_full()
        .overflow_y_scroll()
        .children(results.iter().enumerate().map(|(i, result)| {
            let is_selected = i == selected;
            let el = div()
                .w_full()
                .px(px(16.0))
                .py(px(8.0))
                .flex()
                .items_center()
                .child(
                    div()
                        .child(result.title.clone())
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

fn render_arg_form(
    args: &[CommandArg],
    values: &[String],
    active: usize,
    cursor_pos: usize,
    arg_search: &str,
    arg_choice_idx: usize,
    filtered_choices: &[(String, String)],
) -> impl IntoElement {
    let mut form = div()
        .id("arg-form")
        .flex_1()
        .w_full()
        .px(px(16.0))
        .py(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0));

    for (i, arg) in args.iter().enumerate() {
        let is_active = i == active;
        let value = values.get(i).cloned().unwrap_or_default();
        let is_literal = arg.arg_type == "literal" && !arg.choices.is_empty();
        let label = if arg.required {
            format!("{} *", arg.name)
        } else {
            arg.name.clone()
        };

        let mut field = div()
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .child(label)
                    .text_color(if is_active { rgb(0x89b4fa) } else { rgb(0x585b70) })
                    .text_size(px(12.0))
                    .flex_shrink_0(),
            );

        if is_active {
            field = field.bg(rgb(0x313244));
        } else {
            field = field.bg(rgb(0x181825));
        }

        if is_literal {
            let display_text = if is_active {
                arg_search.to_string()
            } else {
                arg.choices.iter()
                    .find(|c| c.value == value)
                    .map(|c| c.name.clone())
                    .unwrap_or_default()
            };
            if is_active {
                let input = div()
                    .flex()
                    .items_center()
                    .child(div().child(display_text).text_color(rgb(0xf9e2af)).text_size(px(14.0)))
                    .child(div().w(px(1.5)).h(px(16.0)).bg(rgb(0xf9e2af)));
                field = field.child(input);
            } else {
                field = field.child(
                    div().child(if display_text.is_empty() { "...".to_string() } else { display_text })
                        .text_color(rgb(0xa6adc8)).text_size(px(14.0)),
                );
            }
        } else {
            let cpos = if is_active { cursor_pos } else { 0 };
            let before = value[..cpos].to_string();
            let after = value[cpos..].to_string();

            if is_active {
                let input = div()
                    .flex()
                    .items_center()
                    .child(div().child(before).text_color(rgb(0xcdd6f4)).text_size(px(14.0)))
                    .child(div().w(px(1.5)).h(px(16.0)).bg(rgb(0xcdd6f4)))
                    .child(div().child(after).text_color(rgb(0xcdd6f4)).text_size(px(14.0)));
                field = field.child(input);
            } else if value.is_empty() {
                field = field.child(
                    div().child("...").text_color(rgb(0x45475a)).text_size(px(14.0)),
                );
            } else {
                field = field.child(
                    div().child(value).text_color(rgb(0xa6adc8)).text_size(px(14.0)),
                );
            }
        }

        form = form.child(field);

        // Show filtered choices dropdown below the active literal field
        if is_active && is_literal && !filtered_choices.is_empty() {
            let dropdown = div()
                .w_full()
                .ml(px(20.0))
                .flex()
                .flex_col()
                .children(filtered_choices.iter().enumerate().map(|(ci, (name, _))| {
                    let is_highlighted = ci == arg_choice_idx;
                    let mut row = div()
                        .w_full()
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .flex()
                        .items_center()
                        .child(
                            div().child(name.clone())
                                .text_color(if is_highlighted {
                                    rgb(0xf9e2af)
                                } else {
                                    rgb(0xa6adc8)
                                })
                                .text_size(px(13.0)),
                        );
                    if is_highlighted {
                        row = row.bg(rgb(0x45475a));
                    }
                    row
                }));
            form = form.child(dropdown);
        }
    }

    form
}

struct InputHandlerWrapper<V: EntityInputHandler> {
    child: AnyElement,
    focus_handle: FocusHandle,
    entity: Entity<V>,
}

impl<V: EntityInputHandler + 'static> Element for InputHandlerWrapper<V> {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = self.child.request_layout(window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        window.handle_input(
            &self.focus_handle,
            ElementInputHandler::new(bounds, self.entity.clone()),
            cx,
        );
    }
}

impl<V: EntityInputHandler + 'static> IntoElement for InputHandlerWrapper<V> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
