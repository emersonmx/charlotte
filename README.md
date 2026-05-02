# Charlotte

Charlotte is a modern, interactive HTTP/HTTPS proxy and request inspector,
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

Clone the repository:

```sh
git clone https://github.com/emersonmx/charlotte.git
cd charlotte
```

Install development tools and hooks:

```sh
just setup
```

Build the project:

```sh
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

When you start Charlotte for the first time, a custom Certificate Authority (CA)
is automatically generated in your system's default configuration directory, for
example: `~/.config/charlotte/certs/ca.{crt,pem,key}`.

To avoid HTTPS security warnings in browsers or clients, you need to add the
`ca.crt` (or `ca.pem`) file as a trusted CA in your operating system or browser.

## Usage

- The TUI displays incoming HTTP requests and their responses.
- Use arrow keys or `j`/`k` to navigate.
- Press `q` to quit.

## Development

- Format code: `just format`
- Lint code: `just lint`
- Run tests: `just test`
- Run with live reload: `just watch`

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

