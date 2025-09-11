use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{SinkExt, StreamExt};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
};
use std::io::stdout;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

enum AppMode {
    Nickname,
    Chat,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let server_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:9001".to_string());

    let (ws_stream, _) = connect_async(&server_url).await.expect("Failed to connect");
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<String>();
    let messages = Arc::new(Mutex::new(Vec::new()));

    tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            if let Message::Text(text) = msg {
                msg_tx.send(text).unwrap();
            }
        }
    });

    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut mode = AppMode::Nickname;
    let mut input = String::new();
    let mut current_room = "general".to_string();
    let mut scroll: usize = 0;

    loop {
        let size = terminal.size().unwrap();
        let messages_height = size.height.saturating_sub(7) as usize;

        while let Ok(msg) = msg_rx.try_recv() {
            messages.lock().unwrap().push(msg);
        }

        let total_msgs = messages.lock().unwrap().len();
        let max_scroll = total_msgs.saturating_sub(messages_height);
        if scroll > max_scroll {
            scroll = max_scroll;
        }

        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                    .split(f.size());

                let msgs = messages.lock().unwrap();
                let start = if total_msgs > messages_height {
                    total_msgs - messages_height - scroll
                } else {
                    0
                };
                let end = start + messages_height;
                let display_msgs: Vec<Spans> = msgs[start..total_msgs.min(end)]
                    .iter()
                    .map(|m| Spans::from(Span::raw(m.clone())))
                    .collect();

                let messages_widget = Paragraph::new(display_msgs).block(
                    Block::default()
                        .title(format!("Room: {}", current_room))
                        .borders(Borders::ALL),
                );
                f.render_widget(messages_widget, chunks[0]);

                let input_title = match mode {
                    AppMode::Nickname => "Enter nickname",
                    AppMode::Chat => "Input",
                };
                let input_widget = Paragraph::new(format!("❯ {}", input))
                    .block(Block::default().title(input_title).borders(Borders::ALL));
                f.render_widget(input_widget, chunks[1]);
            })
            .unwrap();

        if event::poll(std::time::Duration::from_millis(50)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                            break;
                        } else {
                            input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        let msg = input.drain(..).collect::<String>();
                        if msg.trim().is_empty() {
                            continue;
                        }

                        match mode {
                            AppMode::Nickname => {
                                let nickname = msg.trim().to_string();
                                messages.lock().unwrap().clear();
                                ws_sink.send(Message::Text(nickname.clone())).await.unwrap();
                                mode = AppMode::Chat;
                            }
                            AppMode::Chat => {
                                match msg.trim() {
                                    "/quit" => break,
                                    "/clear" => {
                                        messages.lock().unwrap().clear();
                                        continue;
                                    }
                                    _ => {}
                                }

                                if msg.starts_with("/join ") {
                                    let parts: Vec<&str> = msg.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        current_room = parts[1].to_string();
                                    }
                                }

                                ws_sink.send(Message::Text(msg)).await.unwrap();
                            }
                        }
                    }
                    KeyCode::Esc => break,
                    KeyCode::Up => {
                        if scroll < max_scroll {
                            scroll += 1;
                        }
                    }
                    KeyCode::Down => {
                        if scroll > 0 {
                            scroll -= 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}
