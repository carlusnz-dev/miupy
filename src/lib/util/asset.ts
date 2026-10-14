// @req RF-05
// O WebKitGTK 2.54 bloqueia carga no-cors de um esquema próprio do app (asset://) quando a
// página vem de http(s), como o servidor de dev em http://localhost:1420
// (CachedResourceLoader::isNoCorsCrossOriginRequestToURLSchemeHandler). Com crossorigin a
// imagem vai em modo CORS: a casca registra o esquema como CORS-enabled (asset_webkit.rs) e o
// handler do Tauri devolve Access-Control-Allow-Origin para a janela. URLs https (miniaturas
// do YouTube) ficam sem o atributo, porque não mandam cabeçalho de CORS.
export function assetCrossOrigin(src: string | null | undefined): 'anonymous' | undefined {
  return src && /^(asset:|https?:\/\/asset\.localhost\/)/.test(src) ? 'anonymous' : undefined;
}
