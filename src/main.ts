import App from './App.svelte';
import { mount } from 'svelte';

const target = document.getElementById('app');
if (!target) throw new Error('#app absent de index.html');

export default mount(App, { target });
