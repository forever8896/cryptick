<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { fade, fly, scale, slide } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { cubicOut } from 'svelte/easing';
  import { createEventDispatcher } from 'svelte';

  export let symbol;
  export let removeTicker;
  export let selectedSound;
  
  let price = "Loading";
  let isPositive = true;
  let alertMessages = [];
  let targetPrice = "";
  let alerts = [];
  let duplicateAlertMessage = '';

  $: sortedAlerts = alerts.sort((a, b) => b.price - a.price);

  const dispatch = createEventDispatcher();

  async function startWebSocket() {
    try {
      await invoke("start_websocket", { symbol });
      console.log("WebSocket started for", symbol);
    } catch (error) {
      console.error("Failed to start WebSocket:", error);
    }
  }

  async function addAlert(price) {
    const price_f64 = parseFloat(price);
    if (isNaN(price_f64)) {
      setAlertMessage("Please enter a valid number for the price.");
      return;
    }

    // Check if the alert already exists
    if (alerts.some(alert => Math.abs(alert.price - price_f64) < 0.000001)) {
      duplicateAlertMessage = "Alert already exists for this price.";
      setTimeout(() => {
        duplicateAlertMessage = '';
      }, 3000);
      return;
    }

    try {
      console.log(`Attempting to add alert for ${symbol} at price ${price_f64}`);
      
      await invoke("add_alert", { symbol, price: price_f64 });
      await syncAlerts();
      
      setAlertMessage(`Alert set for price ${price_f64}`);
      targetPrice = "";
      await invoke('save_app_settings');
    } catch (error) {
      console.error("Error in addAlert function:", error);
      setAlertMessage("Failed to add alert. Please try again.");
    }
  }

  async function removeAlert(index) {
    try {
      const alert = alerts[index];
      await invoke("remove_alert", { symbol, price: alert.price });
      await syncAlerts();
      setAlertMessage(`Removed alert for price ${alert.price}`);
      await invoke('save_app_settings');
    } catch (error) {
      console.error("Failed to remove alert:", error);
      setAlertMessage("Failed to remove alert.");
    }
  }

  async function syncAlerts() {
    try {
      const backendAlerts = await invoke("get_alerts", { symbol });
      alerts = backendAlerts.map(alert => ({ price: alert.price }));
      console.log("Synced alerts:", alerts); // Add this line for debugging
    } catch (error) {
      console.error("Failed to sync alerts:", error);
    }
  }

  onMount(async () => {
    await startWebSocket();
    await syncAlerts();

    listen(`price-update-${symbol}`, (event) => {
      const newPrice = parseFloat(event.payload);
      isPositive = newPrice >= parseFloat(price);
      price = formatPrice(newPrice);
    });

    listen(`alert-triggered-${symbol}`, (event) => {
      const triggeredPrices = event.payload;
      setAlertMessage(`Alert triggered for ${symbol}: ${triggeredPrices.join(", ")}`);
      alerts = alerts.filter(alert => !triggeredPrices.includes(alert.price));
      invoke("play_alert_sound", { filename: selectedSound });
    });
  });

  function handleInput(event) {
    const input = event.target;
    const value = input.value;
    
    // Remove any non-numeric characters except for the decimal point
    const sanitizedValue = value.replace(/[^0-9.]/g, '');
    
    // Ensure only one decimal point
    const parts = sanitizedValue.split('.');
    if (parts.length > 2) {
      parts.pop();
    }
    
    const finalValue = parts.join('.');
    
    // Update the input value if it has changed
    if (finalValue !== value) {
      input.value = finalValue;
    }
    
    // Update the bound variable
    targetPrice = finalValue;
  }

  // Modify the functions that set alertMessage to use this new approach
  function setAlertMessage(message) {
    if (message.includes('$')) {
      const parts = message.split('$');
      const prices = parts[1].split(',').map(p => {
        const price = parseFloat(p);
        return isNaN(price) ? p : formatPrice(price);
      });
      message = `${parts[0]}$${prices.join(',')}`;
    }

    const newMessage = { id: Date.now(), text: message };
    alertMessages = [...alertMessages, newMessage];
    
    setTimeout(() => {
      alertMessages = alertMessages.filter(msg => msg.id !== newMessage.id);
    }, 8000);
  }

  function slideUpFade(node, { delay = 0, duration = 300 }) {
    return {
      delay,
      duration,
      css: t => `
        opacity: ${t};
        transform: translateY(${(1 - t) * 10}px);
      `
    };
  }

  function formatPrice(price) {
    if (typeof price === 'string' && price === 'Loading') return price;
    
    const numPrice = parseFloat(price);
    if (isNaN(numPrice)) return price;

    // For prices under $1, show more decimal places
    if (numPrice < 1) {
      return numPrice.toFixed(6);
    } 
    // For prices under $100, show 4 decimal places
    else if (numPrice < 100) {
      return numPrice.toFixed(4);
    }
    // For prices $100 and above, show 2 decimal places
    else {
      return numPrice.toFixed(2);
    }
  }

  function formatAlertPrice(price) {
    return formatPrice(price);
  }
</script>

<div class="ticker-window">
  <button class="remove-ticker-btn" on:click={() => removeTicker(symbol)}>×</button>
  <div class="header" in:fade={{duration: 300, delay: 300}}>
    <h1>{symbol}</h1>
    <div class="price-container">
      <h2 class:positive={isPositive} class:negative={!isPositive}>${price}</h2>
    </div>
  </div>

  <div class="alert-setter" in:fly={{y: 20, duration: 300, delay: 600}}>
    <input
      placeholder="Set alert price"
      bind:value={targetPrice}
      type="text"
      inputmode="decimal"
      on:input={handleInput}
    />
    <button on:click={() => { if (targetPrice) addAlert(targetPrice); }}>
      Set Alert
    </button>
  </div>
  <div class="duplicate-alert-container">
    {#if duplicateAlertMessage}
      <p class="duplicate-alert-message" transition:fade={{duration: 300}}>
        {duplicateAlertMessage}
      </p>
    {/if}
  </div>

  <div class="info" in:fly={{y: 20, duration: 300, delay: 900}}>
    <h3>Active Alerts</h3>
    <div class="alerts-container">
      {#if sortedAlerts.length === 0}
        <p class="no-alerts" in:fade>No active alerts</p>
      {:else}
        <ul>
          {#each sortedAlerts.slice().reverse() as alert (alert.price)}
            <li
              in:slide={{duration: 300}}
              out:slide={{duration: 300}}
            >
              <span class="alert-price">${formatAlertPrice(alert.price)}</span>
              <button class="remove-btn" on:click={() => removeAlert(alerts.indexOf(alert))}>×</button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>

  <div class="alert-message-container">
    {#each alertMessages as message (message.id)}
      <div class="alert-message" 
        transition:slide={{duration: 300}}
      >
        {message.text}
      </div>
    {/each}
  </div>
</div>

<style>
  .ticker-window {
    position: relative;
    width: 100%;
    max-width: 350px;
    min-width: 250px; /* Add this line */
    padding: 1.5rem;
    background-color: var(--secondary-color);
    border-radius: 0.5rem;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    height: 70vh;
    min-height: 400px; /* Add this line */
    font-family: 'Poppins', sans-serif;
    margin: 1rem;
    font-weight: 600;
    overflow: hidden;  /* Add this line to ensure content doesn't overflow */
    user-select: none; /* Prevents text selection while dragging */
  }

  .header {
    margin-bottom: 1.5rem;
    text-align: center;
    flex-shrink: 0;
  }

  h1 {
    font-size: 2rem;
    font-weight: 700;
    margin-bottom: 0.25rem;
    color: var(--text-color);
  }

  h2 {
    font-size: 1.75rem;
    font-weight: 400;
    color: var(--text-color);
  }

  .positive { color: #4caf50; }
  .negative { color: #f44336; }


  .price-container {
   background-color: var(--primary-color);
   border-radius: 0.5rem;
   box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .alert-setter {
    margin-bottom: 0.5rem; /* Reduced from 1.5rem */
    flex-shrink: 0;
    display: flex;
    gap: 0.5rem;
  }

  .alert-setter input {
    flex-grow: 1;
    background-color: var(--primary-color);
    color: var(--text-color);
    border: 2px solid transparent;
    transition: border 0.3s ease;
    border-radius: 0.5rem;
  }

  .alert-setter input:focus, .alert-setter input:hover {
    border: 2px solid var(--accent-color);
  }

  .alert-setter button {
    white-space: nowrap;
    background-color: var(--primary-color);
    color: var(--text-color);
  }

  .info {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    margin-bottom: 20px;  /* Increase this value to make room for the alert message */
  }

  h3 {
    margin-bottom: 0.5rem;
    font-size: 1.1rem;
    color: var(--text-color);
  }

  .alerts-container {
    flex-grow: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0.5rem;
    background-color: var(--primary-color);
    border-radius: 8px;
    margin-bottom: 1rem;
    box-shadow: 0 6px 6px rgba(0, 0, 0, 0.3);
    height: calc(4 * (1.5rem + 0.5rem)); 
  }

  ul {
    margin: 0;
    padding: 0;
    list-style-type: none;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: var(--primary-color);
    border: 1px solid var(--accent-color);
    border-radius: 0.25rem;
    margin-bottom: 0.5rem;
    height: 1.5rem; 
  }

  li:last-child {
    margin-bottom: 0;
  }

  .alert-price {
    flex-grow: 1;
    color: var(--text-color);
    font-size: 0.9rem; /* Slightly smaller font size */
  }

  .remove-btn {
    background-color: transparent;
    color: var(--accent-color);
    font-size: 1rem;
    padding: 0.25rem;
    border: none;
    cursor: pointer;
    transition: color 0.3s ease;
    margin-left: 0.5rem;
  }

  .remove-btn:hover {
    color: var(--primary-color);
  }

  .no-alerts {
    color: var(--text-color);
    text-align: center;
    font-style: italic;
    opacity: 0.7;
  }
  .alert-message-container {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    overflow: hidden;
    height: 40px; /* Set a fixed height */
    /* border: 2px solid var(--accent-color); */
    border-radius: 0.5rem;
    overflow: visible;
    
  }

  .alert-message {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background-color: var(--primary-color);
    color: var(--text-color);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    text-align: center;
    /* padding: 10px; */
    height: 100%;
    border-radius: 0.5rem;
    border: 2px solid var(--accent-color);
   
    
  }

  /* Add this to your :root or adjust the existing --primary-color */
  :root {
    --primary-color-rgb: 36, 10, 52;  /* This should match your --primary-color */

    /* #240A34 to rgb? */
  }

  input, button, select {
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
    box-sizing: border-box;
    font-weight: 600; /* Add this line to increase font weight */
  }

  input, select {
    border: 2px solid var(--accent-color);
    background-color: var(--primary-color);
    color: var(--text-color);
    transition: outline 0.5s ease;
    outline: 1px solid transparent;
  }

  input:focus {
    outline: 1px solid var(--primary-color);
  }

  button {
    background-color: var(--primary-color);
    color: var(--text-color);
    border: none;
    border-radius: 0.25rem;
    cursor: pointer;
    transition: background-color 0.3s ease, color 0.3s ease;
    font-weight: 600; /* Add this line to increase font weight */
  }

  button:hover {
    background-color: var(--accent-color);
    color: var(--primary-color);
  }

  /* Scrollbar styles */
  .alerts-container::-webkit-scrollbar {
    width: 6px;
  }

  .alerts-container::-webkit-scrollbar-track {
    background: var(--primary-color);
  }

  .alerts-container::-webkit-scrollbar-thumb {
    background-color: var(--accent-color);
    border-radius: 20px;
    border: 2px solid var(--primary-color);
  }

  .alerts-container::-webkit-scrollbar-thumb:hover {
    background-color: var(--accent-color);
  }

  /* For Firefox */
  .alerts-container {
    scrollbar-width: thin;
    scrollbar-color: var(--accent-color) var(--primary-color);
  }

  .remove-ticker-btn {
    position: absolute;
    top: 10px;
    right: 10px;
    color: var(--accent-color);
    background-color: transparent;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    padding: 5px;
    line-height: 1;
    transition: color 0.3s ease;
  }

  .remove-ticker-btn:hover {
    color: var(--primary-color);
    background-color: transparent;
  }

  @media (max-width: 768px) {
    .ticker-window {
      padding: 0.75rem;
      margin: 0.5rem;
      height: 65vh; /* Increased from 60vh */
      min-height: 350px;
    }

    .info {
      height: calc(100% - 160px); /* Adjusted for smaller screens */
    }

    h1 {
      font-size: 1.5rem;
    }

    h2 {
      font-size: 1.25rem;
    }

    h3 {
      font-size: 1rem;
    }

    .alert-setter input,
    .alert-setter button {
      font-size: 0.8rem;
    }

    .alerts-container {
      font-size: 0.9rem;
      max-height: calc(4 * (1.3rem + 0.4rem + 2px)); 
    }

    li {
      padding: 0.4rem 0.6rem;
      height: 1.3rem;
      margin-bottom: 0.4rem;
    }

    .alert-price {
      font-size: 0.85rem;
    }

    .remove-btn {
      font-size: 0.9rem;
    }

    .alert-message {
      font-size: 0.85rem;
      padding: 0.6rem;
    }
  }

  @media (max-width: 480px) {
    .ticker-window {
      min-width: 200px;
      height: 55vh; /* Increased from 50vh */
      min-height: 300px;
    }

    .info {
      height: calc(100% - 140px); /* Adjusted for even smaller screens */
    }

    h1 {
      font-size: 1.25rem;
    }

    h2 {
      font-size: 1rem;
    }

    h3 {
      font-size: 0.9rem;
    }

    .alert-setter input,
    .alert-setter button {
      font-size: 0.7rem;
    }

    .alerts-container {
      font-size: 0.8rem;
      max-height: calc(4 * (1.1rem + 0.3rem + 2px)); 
    }

    li {
      padding: 0.3rem 0.5rem;
      height: 1.1rem;
      margin-bottom: 0.3rem;
    }

    .alert-price {
      font-size: 0.75rem;
    }

    .remove-btn {
      font-size: 0.8rem;
    }

    .alert-message {
      font-size: 0.75rem;
      padding: 0.5rem;
    }
  }

  .duplicate-alert-container {
    height: 1.5rem; /* Set a fixed height */
    margin-bottom: 0.5rem;
  }

  .duplicate-alert-message {
    color: var(--accent-color);
    font-size: 0.8rem;
    text-align: center;
    font-style: italic;
    margin: 0;
  }
</style>