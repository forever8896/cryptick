# CrypTick

This is a Tauri application for receiving crypto price alerts, built with Svelte and Vite, created using the `create-tauri-app` command. After reading the 7th chapter of the Rust book, I wanted to jump in and build something.

## Features

- Fetching of crypto prices from the Binance public websocket API
- Real-time crypto price alerts based on set prices
- A couple of fun sounds to choose from

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/forever8896/cryptick.git
   cd cryptick
   ```

2. Install dependencies:

   ```bash
   npm install
   ```

## Development

To run the application in development mode with live-reloading, use the following command:

```bash
npm run tauri dev
```

This will start the Tauri development server and open the application. Any changes you make to the source files will automatically trigger a reload, allowing you to see your changes in real-time.

## Build

To build the application for production:

```bash
npm run tauri build
```

The executable will be located in the `src-tauri/target/release` directory.

## Usage

Run the executable found in `src-tauri/target/release` to start the application.

## Notes

Currently, it only works on desktop, but I chose Tauri with the vision of making it cross-platform in the future.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Contributing

Feel free to open issues or submit pull requests for improvements and bug fixes.
