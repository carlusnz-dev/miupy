// Roteador mínimo por hash (#/caminho). Funciona igual no WebView do Tauri e no navegador.

export type RouteName =
  | 'home' | 'library' | 'playlists' | 'playlist' | 'youtube' | 'downloads'
  | 'now-playing' | 'artist' | 'stats' | 'dna' | 'wrapped' | 'profile' | 'settings' | 'not-found';

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
  if (a === 'dna') return { name: 'dna', params: {} };
  if (a === 'estatisticas') return { name: 'stats', params: {} };
  if (a === 'retrospectiva') return { name: 'wrapped', params: {} };
  if (a === 'perfil') return { name: 'profile', params: {} };
  if (a === 'configuracoes') return { name: 'settings', params: {} };
  return { name: 'not-found', params: {} };
}

class Router {
  current = $state<Route>(parse(typeof location !== 'undefined' ? location.hash : ''));
  // @req RF-43
  fromLink = $state(false);
  canGoBack = $state(false);
  private readonly session = Math.random().toString(36).slice(2);
  private index = 0;
  private nextFromLink: boolean | null = null;
  private replacing = false;
  /** Instante da última troca de tela: o WebKit pode já ter voltado sozinho com o botão do mouse. */
  lastChange = 0;
  constructor() {
    if (typeof window !== 'undefined') {
      this.saveEntry(false);
      window.addEventListener('hashchange', () => {
        const entry = history.state?.miupyNav;
        if (entry?.session === this.session && entry.hash === location.hash) {
          this.index = entry.index;
          this.fromLink = entry.fromLink;
        } else {
          if (!this.replacing) {
            this.index++;
            this.fromLink = this.nextFromLink ?? true;
          }
          this.saveEntry(this.fromLink);
        }
        this.nextFromLink = null;
        this.replacing = false;
        this.lastChange = performance.now();
        this.canGoBack = this.index > 0;
        this.current = parse(location.hash);
      });
    }
  }
  private saveEntry(fromLink: boolean) {
    const state = history.state;
    history.replaceState({ ...(state && typeof state === 'object' ? state : {}), miupyNav: { session: this.session, index: this.index, hash: location.hash, fromLink } }, '');
  }
  menuNavigate(path: string) {
    if (location.hash === href(path) || (!location.hash && path === '/')) {
      this.fromLink = false;
      this.saveEntry(false);
      return;
    }
    this.nextFromLink = false;
    navigate(path);
  }
  /** Troca a tela atual sem criar entrada (ex.: sair de uma playlist excluída). */
  replace(path: string) { if (location.hash === href(path)) return; this.replacing = true; location.replace(href(path)); }
  back() { if (this.canGoBack) history.back(); }
}
export const router = new Router();

export function navigate(path: string) { location.hash = path; }
export function href(path: string) { return `#${path}`; }
export const paths = {
  home: '/', library: '/biblioteca', playlists: '/playlists', youtube: '/youtube', downloads: '/downloads',
  nowPlaying: '/tocando', stats: '/estatisticas', dna: '/dna', wrapped: '/retrospectiva', profile: '/perfil', settings: '/configuracoes',
  playlist: (id: string) => `/playlists/${encodeURIComponent(id)}`,
  artist: (id: string) => `/artista/${encodeURIComponent(id)}`,
};
