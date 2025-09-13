# Rust Chat Client TUI

A terminal-based chat client written in Rust, designed to interact with the WebSocket chat server `leonardo-luz/rust-chat-server`. This client provides a rich text-user interface (TUI) for a seamless chat experience directly from your terminal.

## Features

*   **Interactive TUI:** Built with `ratatui` and `crossterm` for a responsive and engaging terminal interface.
*   **Nickname and Color Selection:** Customize your identity and message color upon joining.
*   **Room Management:** Join different chat rooms using the `/join <room_name>` command.
*   **Real-time Messaging:** Send and receive messages in real-time.
*   **Message History Scrolling:** Scroll through past messages using the Up and Down arrow keys.
*   **Client Count:** See the number of active clients in the current room.
*   **Commands:**
    *   `/quit`: Exit the chat client.
    *   `/clear`: Clear the chat history.
    *   `/join <room_name> [password]`: Join a specified chat room.

## Prerequisites

Before you begin, ensure you have the following installed:

*   **Rust**
*   **Cargo:** Rust's package manager, installed automatically with Rust.
*   **A compatible WebSocket Chat Server:** This client is designed to work with the WebSocket server `leonardo-luz/rust-chat-server`. You will need the server's address to connect.

## Installation

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/leonardo-luz/chat-client.git
    cd chat-client
    ```

2.  **Build the client:**
    ```bash
    cargo build
    ```

## Usage

To run the chat client, you need to provide the WebSocket server's URL as an argument.

```bash
cargo run <websocket_server_url>
```

**Example:**

If your WebSocket server is running locally on `ws://127.0.0.1:9001`:

```bash
cargo run
```

### In-App Interaction

1.  **Nickname:** Upon starting, you will be prompted to enter your desired nickname.
2.  **Color:** Next, enter a 6-digit hexadecimal color code (e.g., `FF0000` for red) for your messages.
3.  **Chat:** You can now send messages.
    *   Type your message and press `Enter` to send.
    *   Use `/join <room_name>` to switch rooms.
    *   Use `Up` and `Down` arrow keys to scroll through messages.
    *   Press `Esc` or `Ctrl+C` to quit.

## Contributing

Contributions are welcome! If you have any suggestions, bug reports, or want to improve the client, feel free to open an issue or submit a pull request.
