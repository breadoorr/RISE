// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://tauri.app/v1/guides/getting-started/setup/sveltekit for more info
export const prerender = true;
export const ssr = false;
