use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, Wrap, Tabs},
    layout::{Layout, Constraint, Direction, Rect},
    style::{Style, Color, Modifier},
    text::Line,
};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{stdout, Result};
use tokio::sync::mpsc::Receiver;
use crate::events::ProxyEvent;
use crate::db::DbStore;

pub enum AppScreen {
    Dashboard,
    Settings,
}

pub enum AppMode {
    Normal,
    Editing,
}

pub struct App {
    pub events: Vec<ProxyEvent>,
    pub db: DbStore,
    pub current_screen: AppScreen,
    pub app_mode: AppMode,
    pub selected_setting: usize,
    pub input_buffer: String,
}

impl App {
    fn handle_save_setting(&mut self) {
        match self.selected_setting {
            0 => {
                let _ = self.db.set_api_key("openai", &self.input_buffer);
                std::env::set_var("OPENAI_API_KEY", &self.input_buffer);
            }
            1 => {
                let _ = self.db.set_api_key("anthropic", &self.input_buffer);
                std::env::set_var("ANTHROPIC_API_KEY", &self.input_buffer);
            }
            2 => {
                let _ = self.db.set_api_key("gemini", &self.input_buffer);
                std::env::set_var("GEMINI_API_KEY", &self.input_buffer);
            }
            3 => {
                let _ = self.db.set_api_key("groq", &self.input_buffer);
                std::env::set_var("GROQ_API_KEY", &self.input_buffer);
            }
            4 => {
                if let Ok(limit) = self.input_buffer.parse::<f64>() {
                    let _ = self.db.set_daily_limit(limit);
                }
            }
            _ => {}
        }
    }
}

pub async fn run_tui(mut rx: Receiver<ProxyEvent>, db: DbStore) -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App {
        events: vec![],
        db,
        current_screen: AppScreen::Dashboard,
        app_mode: AppMode::Normal,
        selected_setting: 0,
        input_buffer: String::new(),
    };

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let CEvent::Key(key) = event::read()? {
                match app.app_mode {
                    AppMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Tab | KeyCode::Char('s') => {
                            app.current_screen = match app.current_screen {
                                AppScreen::Dashboard => AppScreen::Settings,
                                AppScreen::Settings => AppScreen::Dashboard,
                            };
                        }
                        KeyCode::Down => {
                            if matches!(app.current_screen, AppScreen::Settings) {
                                app.selected_setting = (app.selected_setting + 1) % 5;
                            }
                        }
                        KeyCode::Up => {
                            if matches!(app.current_screen, AppScreen::Settings) {
                                app.selected_setting = if app.selected_setting == 0 { 4 } else { app.selected_setting - 1 };
                            }
                        }
                        KeyCode::Enter => {
                            if matches!(app.current_screen, AppScreen::Settings) {
                                app.app_mode = AppMode::Editing;
                                // Pre-fill buffer
                                app.input_buffer = match app.selected_setting {
                                    0 => app.db.get_api_key("openai").unwrap_or(None).unwrap_or_default(),
                                    1 => app.db.get_api_key("anthropic").unwrap_or(None).unwrap_or_default(),
                                    2 => app.db.get_api_key("gemini").unwrap_or(None).unwrap_or_default(),
                                    3 => app.db.get_api_key("groq").unwrap_or(None).unwrap_or_default(),
                                    4 => app.db.get_daily_limit().unwrap_or(5.0).to_string(),
                                    _ => String::new(),
                                };
                            }
                        }
                        _ => {}
                    },
                    AppMode::Editing => match key.code {
                        KeyCode::Enter => {
                            app.handle_save_setting();
                            app.app_mode = AppMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.app_mode = AppMode::Normal;
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        _ => {}
                    },
                }
            }
        }

        while let Ok(proxy_event) = rx.try_recv() {
            app.events.push(proxy_event);
        }

        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);

            let titles = vec!["Dashboard", "Settings"];
            let tab_index = match app.current_screen {
                AppScreen::Dashboard => 0,
                AppScreen::Settings => 1,
            };
            
            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL).title("Navigation (Tab/s to switch, q to quit)"))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .select(tab_index);
            f.render_widget(tabs, chunks[0]);

            match app.current_screen {
                AppScreen::Dashboard => draw_dashboard(f, chunks[1], &app),
                AppScreen::Settings => draw_settings(f, chunks[1], &app),
            }
        })?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn draw_dashboard(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(4),
                Constraint::Min(5),
                Constraint::Length(10),
            ]
            .as_ref(),
        )
        .split(area);

    let daily_cost = app.db.get_daily_cost().unwrap_or(0.0);
    let daily_limit = app.db.get_daily_limit().unwrap_or(5.0);
    
    let openai_active = if std::env::var("OPENAI_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };
    let anthropic_active = if std::env::var("ANTHROPIC_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };
    let gemini_active = if std::env::var("GEMINI_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };
    let groq_active = if std::env::var("GROQ_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };

    let status_line = format!(
        "LLM Proxy Harness | Daily Cost: ${:.4} (Limit: ${:.2})\nAPI Keys: OpenAI {} Anthropic {} Gemini {} Groq {}",
        daily_cost, daily_limit, openai_active, anthropic_active, gemini_active, groq_active
    );

    let header = Paragraph::new(status_line)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(header, chunks[0]);

    let rows: Vec<Row> = app.events.iter().rev().take(15).map(|e| {
        Row::new(vec![
            Cell::from(e.session_id.clone()),
            Cell::from(e.model.clone()),
            Cell::from(e.tokens.to_string()),
            Cell::from(format!("${:.4}", e.cost)),
        ])
    }).collect();

    let table = Table::new(rows, [
        Constraint::Percentage(30),
        Constraint::Percentage(30),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ])
    .header(Row::new(vec!["Session ID", "Model", "Tokens", "Cost"]).style(Style::default().fg(Color::Yellow)))
    .block(Block::default().borders(Borders::ALL).title("Active Sessions"));
    
    f.render_widget(table, chunks[1]);
    
    let mut prompt_texts = String::new();
    for e in app.events.iter().rev().take(3) {
        let clean_prompt = e.prompt_summary.replace('\n', " ");
        prompt_texts.push_str(&format!("[{}]: {}\n", e.session_id, clean_prompt));
    }
    
    let prompts = Paragraph::new(prompt_texts)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Prompt Inspect Mode"));
    f.render_widget(prompts, chunks[2]);
}

fn draw_settings(f: &mut ratatui::Frame, area: Rect, app: &App) {
    let settings_list = vec![
        ("OpenAI API Key", app.db.get_api_key("openai").unwrap_or(None).unwrap_or_default()),
        ("Anthropic API Key", app.db.get_api_key("anthropic").unwrap_or(None).unwrap_or_default()),
        ("Gemini API Key", app.db.get_api_key("gemini").unwrap_or(None).unwrap_or_default()),
        ("Groq API Key", app.db.get_api_key("groq").unwrap_or(None).unwrap_or_default()),
        ("Daily Cost Limit ($)", app.db.get_daily_limit().unwrap_or(5.0).to_string()),
    ];

    let mut list_items = Vec::new();

    for (i, (label, mut value)) in settings_list.into_iter().enumerate() {
        let style = if i == app.selected_setting {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if i == app.selected_setting { ">> " } else { "   " };
        
        if i == app.selected_setting && matches!(app.app_mode, AppMode::Editing) {
            value = format!("{}█", app.input_buffer);
        } else if value.is_empty() {
            value = "(not set)".to_string();
        } else if i != 4 {
            // Mask keys slightly
            if value.len() > 8 {
                value = format!("{}...{}", &value[0..4], &value[value.len()-4..]);
            } else {
                value = "***".to_string();
            }
        }

        let content = format!("{}{}: {}", prefix, label, value);
        list_items.push(Line::styled(content, style));
    }

    let p = Paragraph::new(list_items)
        .block(Block::default().borders(Borders::ALL).title("Settings (Up/Down to select, Enter to edit)"));

    f.render_widget(p, area);
}
