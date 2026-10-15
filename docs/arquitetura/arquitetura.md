# Arquitetura do miupy

O estado atual, módulo a módulo, está em [`arquitetura.json`](arquitetura.json). Este arquivo
explica o desenho e o porquê.

## Visão geral

```
┌──────────────────────────────────────────────────────────────┐
│  src/  — Frontend (Svelte 5 + TS + Tailwind 4)               │
│  telas, stores, tema "magma", cliente IPC tipado             │
└───────────────────────────┬──────────────────────────────────┘
                            │ invoke() / eventos   (docs/arquitetura/ipc.md)
┌───────────────────────────▼──────────────────────────────────┐
│  src-tauri/  — casca do app (única parte que conhece Tauri)  │
│  comandos IPC · estado · orquestração · registro de atividade│
└──────┬──────────────────────┬───────────────────────┬────────┘
       │                      │                       │
┌──────▼────────┐    ┌────────▼────────┐    ┌─────────▼────────┐
│ miupy-core    │    │ miupy-audio     │    │ miupy-media      │
│ domínio       │    │ trait           │    │ trait            │
│ SQLite + migr.│    │  AudioBackend   │    │  MediaResolver   │
│ repositórios  │    │ desktop: rodio  │    │ yt-dlp / rustypipe│
│ estatísticas  │    │ android: Media3 │    │ metadados, tags  │
└──────┬────────┘    └────────┬────────┘    └─────────┬────────┘
       │                      │                       │
   SQLite local         saída de áudio do SO     rede + arquivos
```

## Princípios

1. **As três crates não dependem de Tauri nem umas das outras.** Cada uma define os próprios tipos
   de entrada e saída; `src-tauri` converte entre eles. O custo é um pouco de mapeamento; o ganho é
   testar cada parte com `cargo test`, sem subir janela, e poder reaproveitar o núcleo em outras
   interfaces (CLI, integrações externas — RNF-07).
2. **O que é frágil ou específico de plataforma fica atrás de uma *trait*.** Áudio (`AudioBackend`)
   e extração do YouTube (`MediaResolver`) têm mais de uma implementação e vão ser trocadas.
3. **O banco é a fonte da verdade.** O frontend não guarda estado durável; ele consulta e recebe
   eventos.
4. **Offline-first.** Nada do que toca depende de rede. Rede só em busca, download e enriquecimento,
   e a falha dela nunca quebra a biblioteca.

## Módulos

### `crates/miupy-core`
Domínio e persistência. Modelos (`Track`, `Artist`, `Playlist`, `Profile`, `Settings`), SQLite via
`rusqlite` com migrações versionadas, repositórios, busca (FTS5), ordenação, favoritos, registro de
eventos de reprodução e as consultas de estatísticas e retrospectiva. Detalhes do esquema em
[`banco-de-dados.md`](banco-de-dados.md).

### `crates/miupy-audio`
Trait `AudioBackend` (carregar, tocar, pausar, seek, volume, estado) e a implementação de desktop
sobre `rodio`. O motor roda em uma thread própria, recebe comandos por canal e publica eventos
(posição, fim de faixa, erro). Não conhece fila nem banco: toca um arquivo por vez.

Cadeia de áudio (v0.3): decodificação → ganho da normalização (RF-28) → equalizador de 8 bandas
(RF-29) → volume do usuário → limitador em −1 dBTP. A medição de loudness (EBU R128) também fica
aqui, como função pura sobre um arquivo; a casca decide quando medir e guarda o resultado no banco.

No Android esta implementação não é usada: a reprodução passa por um plugin Tauri em Kotlin com
Media3 (`MediaSessionService`), que é o que permite tocar em segundo plano e aparecer na notificação
e na tela de bloqueio.

### `crates/miupy-media`
Tudo que fala com a internet ou mexe no arquivo de mídia:
- `MediaResolver`: busca no YouTube e download do áudio. Implementação `yt-dlp` (desktop) e, como
  feature opcional, um extrator em Rust para o Android (o `rustypipe` é GPL-3.0 e não serve com a
  licença MIT OR Apache-2.0; escolha em aberto).
- `MetadataProvider`: MusicBrainz, Cover Art Archive e LRCLIB, com limite de requisições.
- `tagger`: leitura e escrita de tags e capa com `lofty`.

### `src-tauri`
A casca. Guarda o estado do app (banco, backend de áudio, fila de reprodução, fila de downloads),
expõe os comandos de [`ipc.md`](ipc.md), emite eventos e faz a orquestração que cruza crates:
- No Linux, habilita CORS para o esquema `asset` no contexto do WebKitGTK. O handler do Tauri
  continua restringindo os arquivos ao escopo configurado em `tauri.conf.json`; isso permite
  mostrar capas tanto com a origem de desenvolvimento HTTP quanto com a origem do app instalado.
- **Fila de reprodução**: próxima/anterior, avanço automático ao fim da faixa.
- **Registro de atividade**: abre um evento de reprodução ao iniciar a faixa e acumula o tempo
  realmente ouvido (pausa não conta) até trocar ou parar.
- **Downloads**: fila com concorrência limitada; baixa, grava tags, salva a capa em WebP, atualiza o banco.
- **Imagens**: codifica uploads, capas e fotos em WebP com perda; migra JPEG/PNG antigos em segundo plano e atualiza os caminhos no banco.

### `src/` (frontend)
SPA em Svelte 5. Camadas: `lib/ipc` (cliente tipado, com *mock* para rodar no navegador sem Tauri),
`lib/stores` (player, biblioteca, tema e o `Resource`, cache por consulta que as telas usam para mostrar
o último dado na hora e atualizar em segundo plano), `lib/components` (inclusive o esqueleto de
carregamento), `routes`. O tema é um conjunto de variáveis CSS trocado pelo atributo `data-lava` no
`<html>`.

## Fluxos principais

**Adicionar do YouTube e baixar**
`youtube_search` → usuário escolhe → `youtube_add_to_library` cria as faixas (sem arquivo) →
`download_enqueue` → resolver baixa M4A → `tagger` grava tags e capa → banco recebe `file_path` e
`download_status = done` → evento `download://progress` e `library://changed`.

**Tocar**
`player_play { trackId, queue }` → casca busca `file_path` no banco → `AudioBackend::load + play` →
abre evento de reprodução → `player://state` e `player://progress` → fim da faixa fecha o evento e
avança a fila.

**Retrospectiva**
`stats_wrapped { year }` → consultas agregadas sobre os eventos de reprodução do ano.

## Dados em disco

Tudo sob o diretório de dados do app (`app_data_dir()` do Tauri):

```
miupy.db          banco SQLite
gpu-startup-pending  sentinela temporária da abertura com GPU forçada em Wayland/NVIDIA
bin/              executáveis gerenciados pelo app (yt-dlp)
audio/            <track_id>.m4a
covers/           <track_id>.webp, albuns/<chave>.webp, artistas/<artist_id>.webp (RNF-12)
avatars/          imagens de perfil enviadas pelo usuário em WebP
playlists/        capas de playlists enviadas pelo usuário em WebP
```

## Android

Reprodução em segundo plano, permissões de notificação e armazenamento, e a escolha do extrator do
YouTube são as três diferenças em relação ao desktop. O resto (banco, domínio, frontend, IPC) é o
mesmo código.
