import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { store } from './lib/store.svelte'

// Apply the saved theme before the first paint of the app.
store.applyTheme()

const target = document.getElementById('app')
if (!target) throw new Error('#app not found')

export default mount(App, { target })
