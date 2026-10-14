# Integração da casca: streaming, recomendações e GPU

- **Data:** 2026-10-13
- **Autor:** shell
- **Modelo:** gpt-6-sol (esforço high), via `codex exec`
- **Escopo:** `src-tauri/` e este relatório; nenhuma crate ou arquivo do frontend foi alterado.

## O que foi feito

- Registrei oito comandos IPC novos: `library_list_tracks_by_artist`, `player_get_queue`, `download_clear_finished`, `stats_wrapped_years`, `stats_taste`, `recommendations_list`, `recommendations_refresh` e `recommendations_dismiss`. Os comandos próprios do app usam a capability `default` existente; o Tauri não exige permissões adicionais para comandos registrados no `invoke_handler`.
- `PlayerState` agora contém `streaming`. Faixas do YouTube sem arquivo usam `resolve_stream` e `AudioBackend::load_url`, com cabeçalhos e duração; faixas locais continuam usando arquivo. A casca renova URL expirada ao retomar e tenta uma nova resolução após o primeiro erro do backend. O segundo erro fecha o evento de reprodução, para a faixa e tenta avançar a fila.
- No download e na atualização de metadados, gravei gêneros, ano, MBID e ISRC recebidos do MusicBrainz. Título, artista, álbum e ano do vídeo complementam campos ausentes; categorias do YouTube não viram gênero. Gênero e ano são enviados ao `TagUpdate`.
- `recommendations_refresh` busca até três sementes por tipo, com até três chamadas simultâneas, tolera falha parcial, guarda candidatas e devolve o ranking local. `recommendations_list` só lê SQLite. Resultados sem duração, menores que 30 segundos ou maiores que 15 minutos são descartados. Adição e dispensa emitem `recommendations://changed` com `{ count }`.
- No Linux, `MIUPY_GPU=auto|on|off` escolhe o renderizador; `auto` desliga DMABUF apenas para Wayland com módulo NVIDIA detectado. `WEBKIT_DISABLE_DMABUF_RENDERER` definido pelo usuário tem prioridade. A decisão e o motivo são registrados no log. A função pura tem teste.
- Atualizei `src-tauri/README.md` e criei um teste de ponta a ponta sem janela usando banco em arquivo temporário, `FakeResolver`, `FakeMetadata` e `NullBackend`.

## Conferência do contrato com o frontend

Li `docs/arquitetura/ipc.md`, `src/lib/ipc/api.ts`, `src/lib/ipc/tauri.ts` e `src/lib/ipc/types.ts`. Os nomes do `invoke`, argumentos camelCase esperados e retornos coincidem:

| Comando | Argumentos no cliente | Retorno |
|---|---|---|
| `library_list_tracks_by_artist` | `artistId`, `query?` | `Track[]` |
| `player_get_queue` | nenhum | `Track[]` |
| `download_clear_finished` | nenhum | `number` |
| `stats_wrapped_years` | nenhum | `number[]` |
| `stats_taste` | nenhum | `TasteProfile` |
| `recommendations_list` | `playlistId?`, `limit?` | `Recommendation[]` |
| `recommendations_refresh` | `playlistId?` | `Recommendation[]` |
| `recommendations_dismiss` | `sourceId` | `void` |

`PlayerState.streaming` é booleano e o evento `recommendations://changed` envia `{ count: number }`, como nos tipos do cliente. Não encontrei divergência contratual que exija editar `ipc.md` ou `src/`.

## Decisões

- O limite de nove sementes reduz o custo de rede; a concorrência de três evita abrir todos os processos `yt-dlp` ao mesmo tempo. Se todas as sementes consultadas falharem por rede, `refresh` devolve `network`; candidatas já salvas permanecem disponíveis por `list`.
- A URL temporária é resolvida pela casca. A renovação após erro é única por faixa; um erro seguinte encerra a reprodução para evitar ciclo infinito. Ao retomar após expiração, a posição é restaurada por `seek`.
- A detecção de GPU usa `/proc/driver/nvidia` ou `/sys/module/nvidia` em sessão Wayland. Ela cobre a combinação relatada sem desligar GPU de todos os usuários; não foi criado marcador de queda anterior.
- Licença, fonte de biografia/foto e distribuição do `yt-dlp` ficaram em aberto. A arquitetura ganhou `recommendations.rs` e `gpu.rs`, mas `docs/arquitetura/arquitetura.json` não foi editado porque esta frente foi proibida de alterá-lo; a orquestração deve atualizá-lo.

## Verificação (comandos rodados e resultado real)

| Comando | Resultado |
|---|---|
| `cargo fmt --all` | Passou. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passou, 0 warnings. |
| `cargo test --workspace` | Passou: casca 74, áudio 26, core 64, media 56 e 1 doctest; 9 testes de rede ignorados por padrão. |
| `git diff --check` | Passou, sem erros de whitespace. |
| `npm run docs:matriz` | Não executado, conforme a regra desta frente; reservado à orquestração. |

Na primeira rodada, um teste antigo ainda esperava `unavailable` para faixa do YouTube sem arquivo e falhou; ele foi atualizado para esperar streaming. Uma rodada intermediária de Clippy apontou construções redundantes, corrigidas antes da verificação final. A verificação final acima foi executada após essas correções.

O teste sem janela **executou**: busca e adição do YouTube, download falso, persistência de gênero/ano, limpeza de jobs, reprodução local até o fim, evento `completed` com tempo ouvido, avanço para streaming simulado, evento `skipped`, estatísticas, retrospectiva e `stats_wrapped_years`, perfil de gosto, atualização e lista ordenada de recomendações com motivos, limite e playlist, erro de rede sem perda do cache, remoção da candidata adicionada à discoteca e dispensa permanente após novo refresh. Testes separados executaram erro `network` ao resolver streaming, renovação de URL expirada, uma recuperação de erro do backend, filtro de vídeos longos e decisão pura da GPU.

## Pendências e problemas conhecidos

- **Janela e efeito visual da GPU não foram vistos.** O autor deve abrir `MIUPY_GPU=on npx tauri dev` e `MIUPY_GPU=off npx tauri dev`, conferir renderização e logs, especialmente em Wayland com NVIDIA.
- Streaming contra URL real do YouTube não foi executado nesta frente. Os testes de integração usam `FakeResolver` e `NullBackend`; o teste de áudio contra rede real fica ignorado por padrão.
- O teste de download usa arquivo de áudio falso: a chamada de escrita das tags foi feita, mas o conteúdo final das tags não foi provado por esse teste. A crate de mídia tem testes próprios de leitura e escrita de tags.
- Não houve execução dos comandos Tauri em uma janela. As funções de domínio chamadas por eles foram exercitadas sem janela, e registro, assinatura e tipos dos comandos passaram pela compilação do workspace.

## Próximos passos

1. A orquestração atualiza o mapa de arquitetura e roda `npm run docs:matriz`.
2. Abrir o aplicativo e conferir os fluxos IPC, a alternância de GPU e o streaming com rede real.
3. Fazer commit e integração pela orquestração; esta frente não executou `git add`, `git commit`, push ou PR.

## Nota da orquestração (revisão antes do merge)

O commit foi feito pela orquestração. Diff lido inteiro; fmt limpo, clippy com 0 warnings e
`cargo test --workspace` com 221 passando, rodados de novo pela orquestração.

A orquestração acrescentou um teste real, ignorado por padrão
(`streaming_real_toca_avanca_e_registra_atividade`): yt-dlp do sistema + backend rodio mudo.
Passou: `play` devolveu em 3,5 s com status `loading`, o som começou 5,7 s depois do clique, a
posição avançou 8 s em 8 s, o seek para 60 s funcionou e o evento de reprodução fechou com 10,5 s
ouvidos.

Problemas conhecidos que ficam:

- `player_play` em streaming resolve a URL com o yt-dlp **antes** de responder e segurando a
  trava do player: são cerca de 3,5 s sem nenhum retorno na interface, e `player_get_state` fica
  bloqueado nesse intervalo. O certo é emitir `loading` na hora e resolver em segundo plano.
- Nesta máquina (Wayland + NVIDIA) o modo `auto` da GPU continua em software; a aceleração só
  liga com `MIUPY_GPU=on`, e ninguém viu o resultado.
