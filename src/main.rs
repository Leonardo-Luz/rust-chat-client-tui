use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::{SinkExt, StreamExt};
use ratatui::style::Color;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::io::{Write, stdin, stdout};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::{WebSocketStream, connect_async, tungstenite::Message};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MessageData {
    msg_type: String,
    sender: String,
    color: String,
    content: String,
    room: String,
    client_count: usize,
}

enum AppMode {
    Nickname,
    Color,
    Chat,
}

async fn connect_ws(
    url: &str,
) -> Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
    match connect_async(url).await {
        Ok((ws_stream, _)) => Some(ws_stream),
        Err(_) => None,
    }
}

fn spawn_receive_task(
    mut ws_stream: futures::stream::SplitStream<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    msg_tx: mpsc::UnboundedSender<MessageData>,
    messages: Arc<Mutex<Vec<MessageData>>>,
) {
    tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            if let Message::Text(text) = msg {
                if let Ok(message_data) = serde_json::from_str::<MessageData>(&text) {
                    let _ = msg_tx.send(message_data);
                }
            }
        }
        messages.lock().unwrap().push(MessageData {
            msg_type: "status".into(),
            sender: "system".into(),
            color: "FF0000".into(),
            content: "Disconnected from server.".into(),
            room: "".into(),
            client_count: 0,
        });
    });
}

async fn send_or_reconnect(
    ws_sink: &mut futures::stream::SplitSink<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >,
    msg: &str,
    tried_reconnect: &mut bool,
    msg_tx: &mpsc::UnboundedSender<MessageData>,
    messages: Arc<Mutex<Vec<MessageData>>>,
    url: &str,
) -> bool {
    if ws_sink.send(Message::Text(msg.to_string())).await.is_ok() {
        return true;
    }

    if !*tried_reconnect {
        if let Some(ws) = connect_ws(url).await {
            let (new_sink, new_stream) = ws.split();
            *ws_sink = new_sink;
            *tried_reconnect = true;

            spawn_receive_task(new_stream, msg_tx.clone(), Arc::clone(&messages));

            // let _ = ws_sink.send(Message::Text(msg.to_string())).await;
            messages.lock().unwrap().push(MessageData {
                msg_type: "status".into(),
                sender: "system".into(),
                color: "00FF00".into(),
                content: "Reconnected to server.".into(),
                room: "".into(),
                client_count: 0,
            });
            return true;
        } else {
            messages.lock().unwrap().push(MessageData {
                msg_type: "status".into(),
                sender: "system".into(),
                color: "FF0000".into(),
                content: format!("Failed to connect to {}. Try again.", url).into(),
                room: "".into(),
                client_count: 0,
            });
        }
    }

    false
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let messages = Arc::new(Mutex::new(Vec::new()));

    let mut server_url = loop {
        print!("Enter WebSocket server URL (default ws://127.0.0.1:9001, q to quit): ");
        stdout().flush().unwrap();

        let mut url = String::new();
        stdin().read_line(&mut url).unwrap();
        let url = url.trim();
        if url.eq_ignore_ascii_case("q") {
            return;
        }
        let url = if url.is_empty() {
            "ws://127.0.0.1:9001"
        } else {
            url
        };

        if let Some(_) = connect_ws(url).await {
            break url.to_string();
        } else {
            println!("Failed to connect to {}. Try again.", url);
        }
    };

    let ws_stream = connect_ws(&server_url).await.unwrap();
    let (mut ws_sink, ws_stream) = ws_stream.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<MessageData>();

    spawn_receive_task(ws_stream, msg_tx.clone(), Arc::clone(&messages));

    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut mode = AppMode::Nickname;
    let mut input = String::new();
    let mut current_room = "general".to_string();
    let mut scroll: usize = 0;
    let mut client_count = 0;
    let mut tried_reconnect = false;

    loop {
        let size = terminal.size().unwrap();
        let messages_height = size.height.saturating_sub(7) as usize;

        while let Ok(msg) = msg_rx.try_recv() {
            if msg.client_count > 0 {
                client_count = msg.client_count;
            }
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
                    total_msgs
                        .saturating_sub(messages_height)
                        .saturating_sub(scroll)
                } else {
                    0
                };
                let end = start + messages_height;
                let display_msgs: Vec<Line> = msgs[start..total_msgs.min(end)]
                    .iter()
                    .map(|m| {
                        let color = if m.color.len() == 6 {
                            let r = u8::from_str_radix(&m.color[0..2], 16).unwrap_or(255);
                            let g = u8::from_str_radix(&m.color[2..4], 16).unwrap_or(255);
                            let b = u8::from_str_radix(&m.color[4..6], 16).unwrap_or(255);
                            Color::Rgb(r, g, b)
                        } else {
                            Color::White
                        };
                        let style = Style::default().fg(color);
                        Line::from(vec![
                            Span::styled(format!("{}: ", m.sender), style),
                            Span::raw(m.content.clone()),
                        ])
                    })
                    .collect();

                let messages_widget = Paragraph::new(display_msgs).block(
                    Block::default()
                        .title(format!("Room: {}[{}]", current_room, client_count))
                        .borders(Borders::ALL),
                );
                f.render_widget(messages_widget, chunks[0]);

                let input_title = match mode {
                    AppMode::Nickname => "Enter nickname",
                    AppMode::Color => "Enter hex color",
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
                        let mut msg = input.drain(..).collect::<String>();
                        if msg.trim().is_empty() {
                            continue;
                        }

                        match mode {
                            AppMode::Nickname => {
                                let _ = send_or_reconnect(
                                    &mut ws_sink,
                                    &msg,
                                    &mut tried_reconnect,
                                    &msg_tx,
                                    Arc::clone(&messages),
                                    &server_url,
                                )
                                .await;
                                mode = AppMode::Color;
                            }
                            AppMode::Color => {
                                let _ = send_or_reconnect(
                                    &mut ws_sink,
                                    &msg,
                                    &mut tried_reconnect,
                                    &msg_tx,
                                    Arc::clone(&messages),
                                    &server_url,
                                )
                                .await;
                                mode = AppMode::Chat;
                            }
                            AppMode::Chat => {
                                if msg == "/quit" {
                                    break;
                                } else if msg == "/clear" {
                                    messages.lock().unwrap().clear();
                                    continue;
                                } else if msg.starts_with("/join ") {
                                    let parts: Vec<&str> = msg.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        current_room = parts[1].to_string();
                                    }
                                } else if msg.starts_with("/color ") {
                                    if let Some(index) = msg.find('#') {
                                        msg.remove(index);
                                    }
                                } else if msg.starts_with("/server ") {
                                    let parts: Vec<&str> = msg.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        server_url = parts[1].to_string();

                                        let _ = ws_sink.send(Message::Close(None)).await;

                                        if let Some(ws) = connect_ws(&server_url).await {
                                            let (new_sink, new_stream) = ws.split();
                                            ws_sink = new_sink;

                                            spawn_receive_task(
                                                new_stream,
                                                msg_tx.clone(),
                                                Arc::clone(&messages),
                                            );

                                            messages.lock().unwrap().push(MessageData {
                                                msg_type: "status".into(),
                                                sender: "system".into(),
                                                color: "00FF00".into(),
                                                content: format!("Connected to {}.", server_url)
                                                    .into(),
                                                room: "".into(),
                                                client_count: 0,
                                            });
                                        } else {
                                            messages.lock().unwrap().push(MessageData {
                                                msg_type: "status".into(),
                                                sender: "system".into(),
                                                color: "FF0000".into(),
                                                content: format!(
                                                    "Failed to connect to {}. Try again.",
                                                    server_url
                                                )
                                                .into(),
                                                room: "".into(),
                                                client_count: 0,
                                            });
                                        }
                                        continue;
                                    }
                                }

                                let _ = send_or_reconnect(
                                    &mut ws_sink,
                                    &msg,
                                    &mut tried_reconnect,
                                    &msg_tx,
                                    Arc::clone(&messages),
                                    &server_url,
                                )
                                .await;
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
