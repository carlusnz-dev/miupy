import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';

// Medidor de fluidez (RNF-13): só no dev, fica fora do bundle de produção.
if (import.meta.env.DEV) void import('./lib/dev/perf');

const app = mount(App, { target: document.getElementById('app')! });
export default app;
