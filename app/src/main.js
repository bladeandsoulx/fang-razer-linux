import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { initBridge } from './lib/bridge.js';

initBridge();

const target = document.getElementById('app');

if (!target) {
  throw new Error('Fang application root is missing');
}

export default mount(App, { target });
