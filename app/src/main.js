import './app.css';
import App from './App.svelte';
import { initBridge } from './lib/bridge.js';

initBridge();

export default new App({ target: document.getElementById('app') });
