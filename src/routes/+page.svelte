<script>
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import TickerWindow from "$lib/TickerWindow.svelte";
  import { fade, fly } from 'svelte/transition';
  import { spring } from 'svelte/motion';
  import { flip } from 'svelte/animate';

  let tickers = [];
  let newSymbol = "";
  let showSettings = false;
  let showInfo = false;
  let selectedSound = "beep.mp3";
  let availablePairs = [];
  let filteredPairs = [];
  let showSuggestions = false;
  let toast = { show: false, message: '', type: 'error' };

  function showToast(message, type = 'error') {
    toast = { show: true, message, type };
    setTimeout(() => {
      toast = { show: false, message: '', type: 'error' };
    }, 3000);
  }

  async function addTicker() {
    if (newSymbol) {
      const symbol = newSymbol.toUpperCase();
      
      if (tickers.includes(symbol)) {
        showToast(`${symbol} is already being tracked`);
        return;
      }

      if (!availablePairs.includes(symbol)) {
        showToast(`${symbol} is not a valid trading pair`);
        return;
      }

      try {
        await invoke("start_websocket", { symbol });
        tickers = [...tickers, symbol];
        console.log("Added new ticker:", symbol);
        newSymbol = "";
        await saveSettings();
      } catch (error) {
        console.error("Failed to add ticker:", error);
        showToast(`Failed to add ${symbol}: ${error}`);
      }
    }
  }

  async function loadSettings() {
    try {
      console.log("Loading app settings");
      const settings = await invoke("get_app_settings");
      console.log("Loaded settings:", settings);
      
      tickers = settings.tickers.map(t => t.symbol);
      console.log("Tickers array set to:", tickers);
      
      selectedSound = settings.selected_sound;

      for (const { symbol, alerts, last_price } of settings.tickers) {
        console.log(`Starting websocket for ${symbol}`);
        await invoke("start_websocket", { symbol });
        await invoke("set_alerts", { symbol, alerts });
        await invoke("set_last_price", { symbol, price: last_price });
      }
    } catch (error) {
      console.error("Failed to load settings:", error);
      tickers = [];
    }
  }

  async function saveSettings() {
    try {
      console.log("Saving app settings");
      await invoke("save_app_settings");
      console.log("Settings saved successfully");
    } catch (error) {
      console.error("Failed to save settings:", error);
    }
  }

  async function removeTicker(symbolToRemove) {
    try {
      await invoke("remove_ticker", { symbol: symbolToRemove });
      tickers = tickers.filter(symbol => symbol !== symbolToRemove);
      console.log(`Removed ticker: ${symbolToRemove}`);
      console.log("Remaining tickers:", tickers);
      await saveSettings();
    } catch (error) {
      console.error(`Failed to remove ticker ${symbolToRemove}:`, error);
    }
  }

  function toggleInfo() {
    showInfo = !showInfo;
  }

  function toggleSettings() {
    showSettings = !showSettings;
  }

  async function soundSelected(sound) {
    selectedSound = sound;
    await setSelectedSound(sound);
  }

  async function setSelectedSound(sound) {
    try {
      await invoke("set_selected_sound", { sound });
      console.log("Selected sound saved:", sound);
      await saveSettings();
    } catch (error) {
      console.error("Failed to set selected sound:", error);
    }
  }

  async function testSound() {
    await invoke("play_alert_sound", { filename: selectedSound });
  }

  async function fetchTradingPairs() {
    try {
      const response = await fetch('https://api.binance.com/api/v3/exchangeInfo');
      const data = await response.json();
      availablePairs = data.symbols
        .filter(symbol => symbol.quoteAsset === 'USDT' && symbol.status === 'TRADING')
        .map(symbol => symbol.symbol);
      console.log('Loaded trading pairs:', availablePairs);
    } catch (error) {
      console.error('Failed to fetch trading pairs:', error);
    }
  }

  function updateSuggestions() {
    if (newSymbol.length > 0) {
      filteredPairs = availablePairs
        .filter(pair => pair.toLowerCase().includes(newSymbol.toLowerCase()))
        .slice(0, 5); // Limit to 5 suggestions
      showSuggestions = filteredPairs.length > 0;
    } else {
      showSuggestions = false;
    }
  }

  function selectSuggestion(pair) {
    newSymbol = pair;
    showSuggestions = false;
  }

  $: {
    console.log('Ticker state changed');
    console.log('Tickers:', tickers);
  }

  onMount(async () => {
    console.log("Component mounted, loading settings");
    await Promise.all([
      loadSettings(),
      fetchTradingPairs()
    ]);
  });
</script>

<svelte:head>
  <link href="https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;700&family=Poppins:wght@400;600&family=Open+Sans:wght@400;600&display=swap" rel="stylesheet">
  <link href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" rel="stylesheet">
</svelte:head>

<div class="app-container">
  <header>
    <div class="header-content">
      
      <img src="/logo.svg" alt="Crypto Ticker Dashboard" class="logo" on:click={toggleInfo} />
      {#if showInfo}
        <div class="info-modal" transition:fade="{{ duration: 300 }}">
          <h2>CrypTick Dashboard</h2>
          <img src="/logo.svg" alt="Crypto Ticker Dashboard" class="logo-info"/>
          <div class="info-content">
            <p>This is a dashboard for tracking crypto prices, utilizing the binance websocket api.</p>
            <p>Start by adding a ticker symbol to the list.</p>
            <p>Then, set an alert for the price you want to track.</p>
            <p>When the price reaches your alert level, you will be notified via sound.</p>
            <p>You can also adjust the alert sound in the settings.</p>
            <br/>
            <div class=contribution>
            <p>Made with ❤️ by <a href="https://brianpistar.dev"  target="_blank">Brian Pistar</a></p>
      
            <p>Built with <a href="https://svelte.dev" target="_blank"><img class="svelte-logo" alt="svelte" src="/svelte.svg" /> Svelte</a> and <a href="https://tauri.app" target="_blank"><img class="tauri-logo" alt="tauri" src="/tauri.svg" /> Tauri</a></p>
          </div>
          </div>
        </div>
      {/if}
      <div class="new-ticker-form">
        <div class="input-button-group">
          <div class="input-wrapper">
            <input
              placeholder="e.g. BTCUSDT, ETHUSDT, etc."
              bind:value={newSymbol}
              on:input={updateSuggestions}
              on:focus={() => updateSuggestions()}
              on:blur={() => setTimeout(() => showSuggestions = false, 200)}
              type="text"
            />
            {#if showSuggestions}
              <div class="suggestions" transition:fade="{{ duration: 100 }}">
                {#each filteredPairs as pair}
                  <div
                    class="suggestion-item"
                    on:mousedown={() => selectSuggestion(pair)}
                  >
                    {pair}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
          <button on:click={addTicker}><i class="fas fa-plus"></i> Add Ticker</button>
        </div>
       
      </div>
      <div class="settings-wrapper">
        
        <button class="settings-btn" on:click={toggleSettings}>
          <i class="fas fa-cog" class:rotate={showSettings}></i>
        </button>
        {#if showSettings}
          <div 
            class="settings-dropdown"
            in:fly="{{ y: -10, duration: 300 }}"
            out:fade="{{ duration: 200 }}"
          >
            <h3>Settings</h3>
            <div class="sound-selector">
              <label for="sound-select">Alert Sound</label>
              <select id="sound-select" bind:value={selectedSound} on:change={() => soundSelected(selectedSound)}>
                <option value="beep.mp3">Beep</option>
                <option value="beep6_dingding.mp3">Ding Ding</option>
                <option value="beep2_loud.mp3">Loud Beep</option>
                <option value="beep3_vibish.mp3">Vibish Beep</option>
                <option value="beep4_voice.mp3">Voice Alert</option>
                <option value="beep5_reporter.mp3">Reporter Voice Alert</option>
              </select>
              <button on:click={testSound}><i class="fas fa-volume-up"></i> Test Sound</button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </header>

 

  <main class="tickers-container-wrapper">
    <div class="tickers-container">
      {#if tickers.length === 0}
        <p class="no-tickers">No tickers added yet. Add a ticker above to get started.</p>
      {:else}
        {#each tickers as symbol (symbol)}
          <div class="ticker-wrapper">
            <TickerWindow {symbol} {removeTicker} {selectedSound} />
          </div>
        {/each}
      {/if}
    </div>
  </main>
</div>

{#if toast.show}
  <div 
    class="toast {toast.type}"
    transition:fade="{{ duration: 200 }}"
  >
    {toast.message}
  </div>
{/if}

<style>
  :global(:root) {


   /* OG VERSION */
    --primary-color: #240A34;
    --secondary-color: #891652;
    --text-color: #FFEDD8;
    --accent-color: #EABE6C;


    /* ATTEMPT 2  */
    /* --primary-color: #363062;
    --secondary-color: #435585;
    --accent-color: #F5E8C7;
    --text-color: #ffffff;   */

    

    /* ATTEMPT 3 */
    /* --primary-color: #181818;
    --secondary-color: #8758FF;
    --accent-color: #5CB8E4;
    --text-color: #ffffff;   */


        /* ATTEMPT 4 */
    /* --primary-color: #382933;
    --secondary-color: #3B5249;
    --accent-color: #519872;
    --text-color: #ffffff;   */


    --primary-color: #000009;
    --secondary-color: #32746D;
    --accent-color: #0FF4C6;
    --text-color: #ffffff;  


      

  }

  :global(body) {
    background-color: var(--primary-color);
    color: var(--text-color);
    font-family: 'Poppins', sans-serif;
    margin: 0;
    padding: 0;
    overflow: hidden; 
    font-weight: 600;
  }

  a {
    color: var(--accent-color);
    text-decoration: none;
    transition: color 0.5s ease;
  }

  a:hover {
    color: var(--secondary-color);
  }

  h1, h2, h3, h4, h5, h6 {
    font-family: 'Poppins', sans-serif;
  }

  h1 {
    font-size: 1.75rem;
    margin-bottom: 1rem;
    color: var(--accent-color);
    font-weight: 600;
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  header {
    background-color: var(--secondary-color);
    padding: 0.5rem 1rem;
    box-shadow: 2px 2px 4px var(--sec-color);
    margin-bottom: 0.5rem;
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    max-width: 1200px;
    margin: 0 auto;
  }

  .logo {
    height: 70px;
    width: auto;
    transition: transform 0.3s ease;
    filter: drop-shadow(2px 2px 4px rgba(0, 0, 0, 0.3));
  }

  .svelte-logo {
    height: 20px;
    width: auto;
    margin-top: 0.25rem;
    margin-left: 0.25rem;
  }

  .tauri-logo {
    height: 20px;
    width: auto;
    margin-top: 0.25rem;
    margin-left: 0.25rem;
  }

  .logo-info {
    height: 100px;
    width: auto;
    transition: transform 0.3s ease;
  }

  .logo:hover {
    transform: scale(1.1);
    cursor: pointer;
  }


  .info-modal {
    position: fixed;
    z-index: 1000;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background-color: var(--primary-color);
    /* border: 1px solid var(--accent-color); */
    border-radius: 0.5rem;
    padding: 1rem;
    width: 80%;
    max-width: 500px;
    max-height: 80vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    box-shadow: 0 0 10px var(--accent-color);
    backdrop-filter: blur(10px);

  }

  .info-modal h2 {
    margin-top: 0;
    color: var(--accent-color);
    text-align: center;
  }

  .info-content {
    flex-grow: 1;
  }

  .info-modal p {
    margin-bottom: 0.5rem;
  }


  .contribution {
    text-align: center;
  }

  /* Remove scrollbar styles */

  .new-ticker-form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 400px;
    margin: 1rem auto;
    padding: 0 1rem;
    color: var(--text-color);
  }

  .input-button-group {
    display: flex;
    gap: 0.5rem;
  }

  .new-ticker-form label {
    font-size: 1rem;
    font-weight: 600;
  }

  .helper-text {
    font-size: 0.8rem;
    opacity: 0.8;
    margin-top: 0.25rem;
  }

  main {
    flex-grow: 1;
    overflow: hidden;
  }

  .tickers-container-wrapper {
    width: 100%;
    height: calc(100vh - 120px);
    overflow-x: auto;
    overflow-y: hidden;
    padding: 1rem;
  }

  .tickers-container {
    display: flex;
    flex-wrap: nowrap;
    gap: 1rem;
    padding: 0.5rem;
    min-height: min-content;
    user-select: none; /* Prevent text selection during drag */
  }

  /* Custom scrollbar styles for Webkit browsers */
  .tickers-container-wrapper::-webkit-scrollbar {
    height: 8px;
  }

  .tickers-container-wrapper::-webkit-scrollbar-track {
    background: var(--primary-color);
  }

  .tickers-container-wrapper::-webkit-scrollbar-thumb {
    background-color: var(--accent-color);
    border-radius: 20px;
    border: 2px solid var(--primary-color);
  }

  /* For Firefox */
  .tickers-container-wrapper {
    scrollbar-width: thin;
    scrollbar-color: var(--accent-color) var(--primary-color);
  }

  input, button, select {
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
    border-radius: 0.25rem;
  }

  input {
    border: 2px solid transparent;
    background-color: var(--primary-color);
    color: var(--text-color);
    flex-grow: 1;
    transition: border 0.3s ease;

  }

  input:focus, input:hover {
    border: 2px solid var(--accent-color);
    outline: none;
  }

  select:focus {
    outline: none;
  }

  button {
    background-color: var(--primary-color);
    color: var(--text-color);
    border: none;
    cursor: pointer;
    transition: background-color 0.5s ease, color 0.5s ease;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    white-space: nowrap;
    font-weight: 600;
  }

  button:hover {
    background-color: var(--accent-color);
    color: var(--primary-color);
  }

  .settings-wrapper {
    position: relative;
  }

  .settings-btn {
    background: none;
    border: none;
    color: var(--primary-color);
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0.5rem;
    transition: all 0.3s ease;
  }

  .settings-btn:hover {
    background: none;
    transform: rotate(45deg);
    color: var(--accent-color);
  }

  .settings-btn i.rotate {
    transform: rotate(180deg);
  }



  .settings-dropdown {
    position: absolute;
    right: 0;
    top: 100%;
    background-color: var(--secondary-color);
    border: 1px solid var(--text-color);
    border-radius: 0.5rem;
    padding: 1rem;
    z-index: 10;
    min-width: 250px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    transform-origin: top right;
  }

  .settings-dropdown h3 {
    margin-top: 0;
    margin-bottom: 1rem;
    color: var(--text-color);
  }

  .sound-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .sound-selector label {
    font-size: 0.9rem;
    color: var(--text-color);
  }

  .sound-selector select,
  .sound-selector button {
    width: 100%;
  }

  select {
    background-color: var(--primary-color);
    color: var(--text-color);
    
  }

  input::placeholder {
    color: var(--text-color);
    opacity: 0.7; /* You can adjust this value to make the placeholder more or less visible */
  }

  /* For Internet Explorer */
  input:-ms-input-placeholder {
    color: var(--text-color);
    opacity: 0.7;
  }

  /* For Microsoft Edge */
  input::-ms-input-placeholder {
    color: var(--text-color);
    opacity: 0.7;
  }

  @media (max-width: 768px) {
    /* ... existing styles ... */

    .tickers-container-wrapper {
      padding-bottom: 8px; /* Add space for the scrollbar */
    }
  }

  /* Add this new style for TickerWindow component */
 

  .input-wrapper {
    position: relative;
    flex-grow: 1;
  }

  .suggestions {
    position: absolute;
    top: calc(100% + 5px); /* Position it 5px below the input */
    left: 0;
    right: 0;
    background-color: var(--primary-color);
    border: 2px solid var(--accent-color);
    border-radius: 0.25rem;
    z-index: 1000;
    max-height: 200px;
    overflow-y: auto;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
  }

  .suggestion-item {
    padding: 0.75rem;
    cursor: pointer;
    transition: all 0.3s ease;
    border-bottom: 1px solid var(--secondary-color);
  }

  .suggestion-item:last-child {
    border-bottom: none;
  }

  .suggestion-item:hover {
    background-color: var(--secondary-color);
    color: var(--accent-color);
  }

  /* Ensure the header has a consistent height */
  .header-content {
    min-height: 80px; /* Adjust this value based on your needs */
  }

  /* Update the new-ticker-form to maintain its position */
  .new-ticker-form {
    position: relative;
    z-index: 1001; /* Ensure it's above other elements */
  }

  .toast {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    background-color: var(--primary-color);
    color: var(--text-color);
    padding: 1rem 2rem;
    border-radius: 0.5rem;
    border: 2px solid var(--accent-color);
    z-index: 2000;
    text-align: center;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .toast.error {
    border-color: #ff4444;
  }

  .toast.success {
    border-color: #00C851;
  }

  .ticker-wrapper {
    height: fit-content;
    cursor: grab;
    transition: all 0.2s ease;
    flex-shrink: 0;
    min-width: 250px;
    touch-action: none;
    position: relative;
  }

  .ticker-wrapper.dragging {
    opacity: 0.5;
    transform: scale(0.95);
    z-index: 100;
  }

  .ticker-wrapper:active {
    cursor: grabbing;
  }

  .no-tickers {
    width: 100%;
    text-align: center;
    color: var(--text-color);
    opacity: 0.7;
    padding: 2rem;
  }

  .ticker-wrapper {
    height: fit-content;
    cursor: grab;
    transition: transform 0.2s ease;
    flex-shrink: 0;
    min-width: 250px;
  }

</style>