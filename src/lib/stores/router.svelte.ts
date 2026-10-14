// Roteador mínimo por hash (#/caminho). Funciona igual no WebView do Tauri e no navegador.

export type RouteName =
  | 'home' | 'library' | 'playlists' | 'playlist' | 'youtube' | 'downloads'
  | 'now-playing' | 'artist' | 'stats' | 'wrapped' | 'profile' | 'settings' | 'not-found';

export interface Route { name: RouteName; params: Record<string, string>; }

function parse(hash: string): Route {
  const path = hash.replace(/^#/, '').split('?')[0] || '/';
  const seg = path.split('/').filter(Boolean).map(decodeURIComponent);
  const [a, b] = seg;
  if (!a) return { name: 'home', params: {} };
  if (a === 'biblioteca') return { name: 'library', params: {} };
  if (a === 'playlists') return b ? { name: 'playlist', params: { id: b } } : { name: 'playlists', params: {} };
  if (a === 'youtube') return { name: 'youtube', params: {} };
  if (a === 'downloads') return { name: 'downloads', params: {} };
  if (a === 'tocando') return { name: 'now-playing', params: {} };
  if (a === 'artista' && b) return { name: 'artist', params: { id: b } };
  if (a === 'estatisticas') return { name: 'stats', params: {} };
  if (a === 'retrospectiva') return { name: 'wrapped', params: {} };
  if (a === 'perfil') return { name: 'profile', params: {} };
  if (a === 'configuracoes') return { name: 'settings', params: {} };
  return { name: 'not-found', params: {} };
}

class Router {
  current = $state<Route>(parse(typeof location !== 'undefined' ? location.hash : ''));
  constructor() {
    if (typeof window !== 'undefined') {
      window.addEventListener('hashchange', () => { this.current = parse(location.hash); });
    }
  }
}
export const router = new Router();

export function navigate(path: string) { location.hash = path; }
export function href(path: string) { return `#${path}`; }
export const paths = {
  home: '/', library: '/biblioteca', playlists: '/playlists', youtube: '/youtube', downloads: '/downloads',
  nowPlaying: '/tocando', stats: '/estatisticas', wrapped: '/retrospectiva', profile: '/perfil', settings: '/configuracoes',
  playlist: (id: string) => `/playlists/${encodeURIComponent(id)}`,
  artist: (id: string) => `/artista/${encodeURIComponent(id)}`,
};
