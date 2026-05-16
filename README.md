# Charlene

Charlene is a modern, interactive HTTP/HTTPS proxy and request inspector,
featuring a TUI (Terminal User Interface) for real-time monitoring of HTTP
traffic. It is designed for developers and testers who need to inspect, debug,
or analyze HTTP(S) requests and responses between clients and servers.

## Features

- **Intercept HTTP and HTTPS traffic** using a custom CA certificate.
- **Live request/response inspection** in a terminal-based UI.
- **Request/response details**: method, URL, headers, body, and status.
- **Easy to use**: minimal configuration, works out of the box.
- **Cross-platform**: runs on Linux, macOS, and Windows (with compatible terminal).

## Getting Started

### Prerequisites

- [Rust](https://rust-lang.org)
- [Go](https://go.dev/) (for installing `lefthook` via `just setup`)
- [just](https://github.com/casey/just) (for running project tasks)

### Installation

Install Charlene directly with Cargo (no need to clone the repository):

```sh
cargo install --locked --git https://github.com/emersonmx/charlene
```

For development (optional), clone the repository and set up tools:

```sh
git clone https://github.com/emersonmx/charlene.git
cd charlene
just setup
just build
```

### Running the Proxy UI

Start the TUI proxy interface:

```sh
just run
```

By default, the proxy listens on `127.0.0.1:8888`. You can change the host and port:

```sh
just run -- --host 0.0.0.0 --port 8080
```

Configure your browser or HTTP client to use the proxy address.

### Certificates

When you start Charlene for the first time, a custom Certificate Authority (CA)
is automatically generated in your system's default configuration directory, for
example: `~/.config/charlene/certs/ca.{crt,pem,key}`.

To avoid HTTPS security warnings in browsers or clients, you need to add the
`ca.crt` (or `ca.pem`) file as a trusted CA in your operating system or browser.

## Usage

- The TUI displays incoming HTTP requests and their responses.
- Use arrow keys or `j`/`k` to navigate.
- Press `q` to quit.
  **Now, a confirmation modal will appear:**
  _"Are you sure you want to quit? (y/n)"_
  Press `y` to confirm or `n` to cancel.

### Keyboard Shortcuts

| Key                | Action                                 |
|--------------------|----------------------------------------|
| Up / k             | Move Up                                |
| Down / j           | Move Down                              |
| Left / h           | Move Left                              |
| Right / l          | Move Right                             |
| Enter              | Select/Confirm                         |
| q                  | Show quit confirmation modal           |
| y                  | Confirm quit (when modal is open)      |
| n                  | Cancel quit (when modal is open)       |
| esc/backspace      | Back/Close modal                       |
| tab / ]            | Next Tab                               |
| s-tab / [          | Previous Tab                           |
| ? / F1             | Show shortcuts help                    |

### Example: Quitting the App

1. Press `q`
2. A modal appears:
   _Are you sure you want to quit? (y/n)_
3. Press `y` to quit, or `n` to return to the app.

## Development

- Format code: `just format`
- Lint code: `just lint`
- Run tests: `just test`
- Run with live reload: `just watch`

## Architecture & Main Components

Charlene is organized into modular components, each with clear responsibilities,
to provide HTTP/HTTPS proxy functionality with a TUI. The main modules are:

- **proxy/**: Handles the proxy server, certificate management, and HTTP layer
  (in `proxy/src/`).
  - `server.rs`: Accepts TCP connections, manages the request/response
    lifecycle, and sends events to the TUI.
  - `certs.rs`: Generates and manages CA and per-domain certificates for HTTPS
    interception.
  - `http.rs`: Defines types and utilities for requests, responses, and header
    manipulation.

- **tui/**: Implements the terminal user interface, divided into screens,
  modals, widgets, and configuration.
  - `tui/src/app.rs`: Manages the application lifecycle, message routing, and
    screen transitions.
  - `tui/src/screens/`: Main screens (requests, HTTP client, error log, etc).
  - `tui/src/modals/`: Transient modals (error, confirmation, shortcuts, etc).
  - `tui/src/widgets.rs`: Custom widgets for the TUI.
  - `tui/src/clipboard.rs`: Clipboard abstraction.
  - `tui/src/config.rs`: Loads and validates configuration.

- **formatter/**: Utilities for formatting JSON and text for display in the TUI.

Communication between the proxy and the TUI uses asynchronous channels
(`tokio::mpsc`), so proxy events (requests, responses, errors) are reflected in
real time in the interface.

Each component is responsible for a well-defined part of the application flow,
making maintenance, testing, and extensibility easier.

## Contributing

Contributions are welcome! Please:

1. Fork the repository and create your branch.
2. Follow the code style (`just format`).
3. Run tests and ensure everything passes.
4. Open a pull request with a clear description.

## License

MIT License. See [LICENSE](LICENSE) for details.
```
```

