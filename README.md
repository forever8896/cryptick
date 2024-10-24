# CrypTick

This is a Tauri application for receiving crypto price alerts, built with Svelte and Vite, Built from the create-tauri-app command. With the 7th chapter of the rust book read, I wanted to jump in and build something.

## Features

- Fetching of crypto prices from the Binance public websocket API
- Real-time crypto price alerts based on set prices.
- A couple fun sounds to choose from.

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

3. Build the application:

   ```bash
   npm run tauri build
   ```

4. Find the executable in the `src-tauri/target/release` directory.

## Usage

Run the executable found in `src-tauri/target/release` to start the application.

## Notes

Currently it only works on desktop, But I chose Tauri with the vision of making it cross platform in the future.


## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Contributing

Feel free to open issues or submit pull requests for improvements and bug fixes.
