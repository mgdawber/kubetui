use std::error::Error;

use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::app::{App, AppState};
use tui::widgets::ListState;

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(());
            }

            match app.state {
                AppState::MainMenu => handle_main_menu(&mut app, key.code),
                AppState::NamespaceSelection => handle_namespace_selection(&mut app, key.code),
                AppState::ContextSelection => handle_context_selection(&mut app, key.code),
                AppState::ExecPodSelection => handle_exec_pod_selection(&mut app, key.code),
                AppState::PodSelection => handle_copy_pod_selection(&mut app, key.code),
                AppState::CopyPodNameInput => handle_copy_pod_name(&mut app, key.code),
                AppState::Message | AppState::ShowOutput => {
                    app.state = AppState::MainMenu;
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(f.size());

    let header = Paragraph::new(format!(
        " Context: {} | Namespace: {} ",
        app.selected_context.as_deref().unwrap_or("None"),
        app.current_namespace()
    ))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, vertical_chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(vertical_chunks[1]);

    let command_items: Vec<ListItem> = app
        .commands
        .iter()
        .map(|c| ListItem::new(c.as_str()))
        .collect();

    let commands = tui::widgets::List::new(command_items)
        .block(Block::default().borders(Borders::ALL).title("Commands"))
        .highlight_symbol("▶");
    f.render_stateful_widget(commands, main_chunks[0], &mut app.list_state);

    match app.state {
        AppState::MainMenu => {
            render_output_preview(f, app, main_chunks[1]);
        }
        AppState::NamespaceSelection => render_list_panel(
            f,
            main_chunks[1],
            &app.namespaces,
            &mut app.namespace_list_state,
            "Select Namespace",
        ),
        AppState::ContextSelection => render_list_panel(
            f,
            main_chunks[1],
            &app.contexts,
            &mut app.context_list_state,
            "Select Context",
        ),
        AppState::ExecPodSelection => render_list_panel(
            f,
            main_chunks[1],
            &app.pods,
            &mut app.pod_list_state,
            "Select Pod to Exec",
        ),
        AppState::PodSelection => render_list_panel(
            f,
            main_chunks[1],
            &app.pods,
            &mut app.pod_list_state,
            "Select Pod to Copy",
        ),
        AppState::CopyPodNameInput => render_copy_pod_ui(f, app, main_chunks[1]),
        AppState::ShowOutput => render_output_panel(f, app, main_chunks[1]),
        AppState::Message => render_message_panel(f, app, main_chunks[1]),
    }

    let status = match app.state {
        AppState::NamespaceSelection
        | AppState::ContextSelection
        | AppState::ExecPodSelection
        | AppState::PodSelection => "[↑/↓ or j/k] Navigate  [Enter/Right] Select  [Esc] Back  [q] Quit",
        AppState::CopyPodNameInput => "[Enter] Submit  [Esc] Back  [q] Quit",
        AppState::Message | AppState::ShowOutput => "Press any key to return to main menu, or [q] Quit",
        AppState::MainMenu => "[↑/↓ or j/k] Navigate  [Enter/Right] Select  [q] Quit",
    };
    let status_bar = Paragraph::new(status).block(Block::default().borders(Borders::TOP));
    f.render_widget(status_bar, vertical_chunks[2]);
}

fn render_output_preview<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let selected = app.list_state.selected().unwrap_or(0);
    if selected == 2 && !app.output.is_empty() {
        let output = Paragraph::new(app.output.as_str())
            .wrap(tui::widgets::Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Pods Preview"));
        f.render_widget(output, area);
    } else {
        render_default_panel(f, area);
    }
}

fn render_list_panel<B: Backend>(
    f: &mut Frame<B>,
    area: Rect,
    items: &[String],
    state: &mut ListState,
    title: &str,
) {
    let list_items: Vec<ListItem> = items.iter().map(|i| ListItem::new(i.as_str())).collect();
    let list = tui::widgets::List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_symbol("▶");
    f.render_stateful_widget(list, area, state);
}

fn render_copy_pod_ui<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    use tui::layout::{Constraint, Direction, Layout};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    let info = Paragraph::new(format!(
        "Copying pod: {}\nEnter new pod name:",
        app.selected_pod.as_deref().unwrap_or("None")
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(info, chunks[0]);

    let input = Paragraph::new(app.new_pod_name.as_str())
        .block(Block::default().borders(Borders::ALL).title("New Pod Name"));
    f.render_widget(input, chunks[1]);

    f.set_cursor(
        chunks[1].x + app.new_pod_name.len() as u16 + 1,
        chunks[1].y + 1,
    );
}

fn render_output_panel<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let output = Paragraph::new(app.output.as_str())
        .wrap(tui::widgets::Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Output"));
    f.render_widget(output, area);
}

fn render_message_panel<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let message = Paragraph::new(app.message.as_str())
        .wrap(tui::widgets::Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Message"));
    f.render_widget(message, area);
}

fn render_default_panel<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Welcome");
    f.render_widget(block, area);
}

fn handle_main_menu(app: &mut App, key_code: KeyCode) {
    let old_index = app.list_state.selected().unwrap_or(0);
    let last_idx = app.commands.len().saturating_sub(1);

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            let new_idx = old_index.saturating_sub(1);
            app.list_state.select(Some(new_idx));
            maybe_load_preview(app, new_idx);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let new_idx = if old_index < last_idx { old_index + 1 } else { 0 };
            app.list_state.select(Some(new_idx));
            maybe_load_preview(app, new_idx);
        }
        KeyCode::Right | KeyCode::Enter => match old_index {
            0 => handle_load_contexts(app),
            1 => handle_load_namespaces(app),
            2 => {
                if let Err(e) = app.load_pods() {
                    app.message = format!("Error loading pods: {}", e);
                    app.state = AppState::Message;
                } else {
                    app.state = AppState::ExecPodSelection;
                }
            }
            3 => {
                match app.load_pods() {
                    Ok(_) => app.state = AppState::PodSelection,
                    Err(e) => {
                        app.message = format!("Error loading pods: {}", e);
                        app.state = AppState::Message;
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn maybe_load_preview(app: &mut App, new_idx: usize) {
    if app.last_main_menu_index == Some(new_idx) {
        return;
    }

    app.output.clear();

    if new_idx == 2 {
        let namespace = app.current_namespace();
        let res = app.execute_kubectl(&["get", "pods", "-n", &namespace]);
        if let Err(e) = res {
            app.output = format!("Error listing pods: {}", e);
        }
    }

    app.last_main_menu_index = Some(new_idx);
}

fn handle_load_namespaces(app: &mut App) {
    if let Err(e) = app.load_namespaces() {
        app.message = format!("Error loading namespaces: {}", e);
        app.state = AppState::Message;
    } else {
        app.state = AppState::NamespaceSelection;
    }
}

fn handle_load_contexts(app: &mut App) {
    if let Err(e) = app.load_contexts() {
        app.message = format!("Error loading contexts: {}", e);
        app.state = AppState::Message;
    } else {
        app.state = AppState::ContextSelection;
    }
}

fn handle_exec_pod_selection(app: &mut App, key_code: KeyCode) {
    let selected = app.pod_list_state.selected().unwrap_or(0);
    let last_idx = app.pods.len().saturating_sub(1);

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.pod_list_state.select(Some(selected.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.pod_list_state.select(Some(if selected < last_idx {
                selected + 1
            } else {
                0
            }));
        }
        KeyCode::Enter => {
            let pod = app.pods.get(selected).cloned();
            if let Some(chosen_pod) = pod {
                if let Err(e) = app.exec_pod(&chosen_pod) {
                    app.message = format!("Error exec into pod: {}", e);
                    app.state = AppState::Message;
                }
            }
        }
        KeyCode::Esc => {
            app.state = AppState::MainMenu;
        }
        _ => {}
    }
}

fn handle_copy_pod_selection(app: &mut App, key_code: KeyCode) {
    let selected = app.pod_list_state.selected().unwrap_or(0);
    let last_idx = app.pods.len().saturating_sub(1);

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.pod_list_state.select(Some(selected.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.pod_list_state.select(Some(if selected < last_idx {
                selected + 1
            } else {
                0
            }));
        }
        KeyCode::Enter => {
            let pod = app.pods.get(selected).cloned();
            if let Some(cloned_pod) = pod {
                app.selected_pod = Some(cloned_pod);
                app.new_pod_name.clear();
                app.state = AppState::CopyPodNameInput;
            }
        }
        KeyCode::Esc => {
            app.state = AppState::MainMenu;
        }
        _ => {}
    }
}

fn handle_copy_pod_name(app: &mut App, key_code: KeyCode) {
    match key_code {
        KeyCode::Enter => {
            if app.new_pod_name.is_empty() {
                app.message = "Please enter a new pod name".to_string();
                app.state = AppState::Message;
            } else if let Some(op) = app.selected_pod.clone() {
                let new_name = app.new_pod_name.clone();
                let result = app.copy_pod(&op, &new_name);

                if let Err(e) = result {
                    app.message = format!("Error copying pod: {}", e);
                    app.state = AppState::Message;
                }
                app.selected_pod = None;
                app.new_pod_name.clear();
            }
        }
        KeyCode::Char(c) => {
            app.new_pod_name.push(c);
        }
        KeyCode::Backspace => {
            app.new_pod_name.pop();
        }
        KeyCode::Esc => {
            app.state = AppState::PodSelection;
        }
        _ => {}
    }
}

fn handle_namespace_selection(app: &mut App, key_code: KeyCode) {
    let selected = app.namespace_list_state.selected().unwrap_or(0);
    let last_idx = app.namespaces.len().saturating_sub(1);

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.namespace_list_state.select(Some(selected.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.namespace_list_state.select(Some(if selected < last_idx {
                selected + 1
            } else {
                0
            }));
        }
        KeyCode::Enter => {
            let ns = app.namespaces.get(selected).cloned();
            if let Some(ns) = ns {
                app.selected_namespace = Some(ns);
                app.state = AppState::MainMenu;
            }
        }
        KeyCode::Esc => {
            app.state = AppState::MainMenu;
        }
        _ => {}
    }
}

fn handle_context_selection(app: &mut App, key_code: KeyCode) {
    let selected = app.context_list_state.selected().unwrap_or(0);
    let last_idx = app.contexts.len().saturating_sub(1);

    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.context_list_state.select(Some(selected.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.context_list_state.select(Some(if selected < last_idx {
                selected + 1
            } else {
                0
            }));
        }
        KeyCode::Enter => {
            let ctx = app.contexts.get(selected).cloned();
            if let Some(context_string) = ctx {
                if let Err(e) = app.switch_context(&context_string) {
                    app.message = format!("Error switching context: {}", e);
                    app.state = AppState::Message;
                } else {
                    app.state = AppState::MainMenu;
                }
            }
        }
        KeyCode::Esc => {
            app.state = AppState::MainMenu;
        }
        _ => {}
    }
}
