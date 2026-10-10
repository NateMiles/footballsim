import { mount } from 'svelte';
import App from './App.svelte';
import { connect } from './api';
import { GameState, gameContext } from './state/game.svelte';
import { defaultSaveStore } from './storage/saves';
import { registerServiceWorker } from './pwa';
import './styles/app.css';

const game = new GameState(connect(), defaultSaveStore());
mount(App, { target: document.getElementById('app')!, context: gameContext(game) });
registerServiceWorker();
