use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, Paragraph, Table, Row, Cell, Wrap},
    layout::{Layout, Constraint, Direction},
    style::{Style, Color, Modifier},
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

pub struct App {
    pub events: Vec<ProxyEvent>,
    pub db: DbStore,
}

pub async fn run_tui(mut rx: Receiver<ProxyEvent>, db: DbStore) -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App {
        events: vec![],
        db,
    };

    loop {
        // Poll for events from channel or crossterm
        if event::poll(std::time::Duration::from_millis(100))? {
            if let CEvent::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        while let Ok(proxy_event) = rx.try_recv() {
            app.events.push(proxy_event);
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(4),
                        Constraint::Min(5),
                        Constraint::Length(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let daily_cost = app.db.get_daily_cost().unwrap_or(0.0);
            
            // Check active keys dynamically
            let openai_active = if std::env::var("OPENAI_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };
            let anthropic_active = if std::env::var("ANTHROPIC_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };
            let gemini_active = if std::env::var("GEMINI_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };
            let groq_active = if std::env::var("GROQ_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) { "🟢" } else { "🔴" };

            let status_line = format!(
                "LLM Proxy Harness | Daily Cost: ${:.4} (Limit: $5.00) | Press 'q' to quit\nAPI Keys: OpenAI {} Anthropic {} Gemini {} Groq {}",
                daily_cost, openai_active, anthropic_active, gemini_active, groq_active
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
            
            // Prompt inspect mode placeholder
            let mut prompt_texts = String::new();
            for e in app.events.iter().rev().take(3) {
                let clean_prompt = e.prompt_summary.replace('\n', " ");
                prompt_texts.push_str(&format!("[{}]: {}\n", e.session_id, clean_prompt));
            }
            
            let prompts = Paragraph::new(prompt_texts)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Prompt Inspect Mode"));
            f.render_widget(prompts, chunks[2]);
            
        })?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
