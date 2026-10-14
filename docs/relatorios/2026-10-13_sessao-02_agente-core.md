# Sessão 02 — núcleo, metadados e recomendações

- **Data:** 2026-10-13
- **Autor:** agente core
- **Modelo:** gpt-6-sol (esforço high), via `codex exec`
- **Escopo:** migração 0002, lacunas do contrato da crate e ranking local RF-19.

## O que foi feito

- Migração 0002 acrescenta `genres`, `release_year`, `mbid` e `isrc` em `tracks`, sem alterar a
  0001. Teste migra banco com artista, faixa e evento existentes.
- `Track` serializa `genres` e `releaseYear` em camelCase. Criação e edição normalizam gêneros
  para minúsculas, sem vazios ou repetição. `NewTrack` e `TrackPatch` aceitam os novos metadados.
- Consultas novas: faixas por artista com `TrackQuery`, resolução de IDs da fila na ordem recebida
  e anos de reprodução pelo início do evento no fuso local. Limpeza de downloads remove `done` e
  `error`, preservando faixa e arquivo.
- Tabela de candidatas por `source_id`, com semente, gêneros, instante de busca e lápide de
  dispensa permanente. Trigger remove candidata ativa quando o vídeo entra em `tracks`.
- Perfil global ou de playlist, sementes limitadas e ranking local com escuta, conclusão, pulo,
  favorito, repetição, recência e diversidade de artistas. Fórmula e pesos em
  [`recomendacao.md`](../arquitetura/recomendacao.md). A crate não faz chamadas de rede.
- Esquema e API documentados em [`banco-de-dados.md`](../arquitetura/banco-de-dados.md) e no
  [README da crate](../../crates/miupy-core/README.md).

### API pública nova para a casca

```rust
Database::list_tracks_by_artist(&self, artist_id: &str, query: &TrackQuery) -> Result<Vec<Track>>
Database::get_tracks_by_ids(&self, track_ids: &[String]) -> Result<Vec<Track>>
Database::wrapped_years(&self, tz_offset_minutes: i32) -> Result<Vec<i32>>
Database::taste_profile(&self, playlist_id: Option<&str>) -> Result<TasteProfile>
Database::recommendation_seeds(&self, playlist_id: Option<&str>, track_limit: u32,
    artist_limit: u32, genre_limit: u32) -> Result<Vec<RecommendationSeed>>
Database::upsert_recommendation_candidate(&self, candidate: &RecommendationCandidate) -> Result<bool>
Database::dismiss_recommendation(&self, source_id: &str) -> Result<()>
Database::list_recommendations(&self, playlist_id: Option<&str>, limit: Option<u32>) -> Result<Vec<Recommendation>>
```

Tipos novos exportados: `Recommendation`, `RecommendationReason`, `RecommendationReasonKind`,
`RecommendationCandidate`, `RecommendationSeed`, `TasteProfile`, `TasteArtist` e `TasteGenre`.
`RecommendationSeed.track_id` identifica a faixa de uma semente `similarTrack`; a casca copia o
valor para `RecommendationCandidate.seed_track_id`. `upsert_recommendation_candidate` retorna
`false` se o vídeo já está na discoteca ou foi dispensado. `list_recommendations` não usa rede;
a casca deve buscar vídeos e guardar candidatas antes de chamá-lo em `recommendations_refresh`.

### Mudanças que quebram consumidores da crate

- `Track` ganhou campos públicos `genres: Vec<String>` e `release_year: Option<i32>`.
- `NewTrack` ganhou `genres`, `release_year`, `mbid` e `isrc`; literais completos precisam desses
  campos. Os construtores `youtube` e `local` continuam com as mesmas assinaturas e inicializam
  os campos novos com valores vazios.
- `TrackPatch` ganhou `genres`, `release_year`, `mbid` e `isrc`; literais completos precisam
  completar esses campos ou usar `..Default::default()`.
- Nenhuma assinatura pública existente foi alterada. `clear_finished_downloads` mudou o
  comportamento: também remove jobs `error`.

## Decisões

- O perfil de uma playlist usa todo o histórico das faixas atualmente nela, mesmo quando foram
  tocadas fora da playlist. Eventos abertos não influem no gosto.
- A dispensa grava uma lápide mesmo para ID ainda não conhecido; o trigger conserva a lápide.
- O ranking limita três candidatas por artista, portanto pode devolver menos que `limit`.
- Licença, fonte de biografia/foto e distribuição do `yt-dlp` continuam em aberto; esta frente
  não fixa nenhuma dessas decisões.

## Verificação (comandos rodados e resultado real)

| Comando | Resultado |
|---|---|
| `cargo fmt --all` | passou, saída vazia |
| `cargo clippy -p miupy-core --all-targets -- -D warnings` | passou, 0 warnings |
| `cargo test -p miupy-core` | 64 testes passaram, 0 falhas; doc-tests 0 |
| `git diff --check` | passou, sem erro de espaços |
| `git add -A && git diff --cached --check && git status --short && git commit -m "Implementa recomendações locais e metadados no núcleo"` | falhou no primeiro comando: `fatal: Unable to create '/home/cabeto/Documentos/Projetos/miupy/.git/worktrees/s02-core/index.lock': Read-only file system`; nada foi commitado |

Os testes rodados cobrem migração com dados, serialização, consultas novas, limpeza de jobs e
ranking (escuta, pulo, favorito, recência, dispensa, discoteca, playlist, limite e banco vazio).
Não foi executado fluxo de rede ou a janela Tauri. `npm run docs:matriz` e as verificações do
workspace inteiro foram reservados à orquestração, como pedido nesta frente.

## Pendências e problemas conhecidos

- A casca precisa integrar os métodos públicos aos comandos IPC e preencher gêneros e ano após
  download ou atualização de metadados. Esta frente só escreveu e testou a crate.
- `RecommendationCandidate` guarda uma semente por vídeo; buscas posteriores atualizam a semente
  da candidata ativa. O perfil e os gêneros da candidata ainda oferecem múltiplos motivos.
- O sandbox impediu escrita no índice Git deste worktree (`index.lock` em sistema de arquivos
  somente leitura). Não houve tentativa de contornar a permissão; arquivos e relatório permanecem
  no worktree sem commit.

## Próximos passos

1. Integrar a API na casca e verificar o contrato IPC ponta a ponta.
2. Rodar a matriz e as verificações do workspace na orquestração.
3. Fazer `git add -A` e o commit quando o índice Git do worktree estiver gravável.
