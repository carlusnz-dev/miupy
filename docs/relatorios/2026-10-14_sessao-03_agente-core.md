# Núcleo: DNA, aleatório e imagens

- **Data:** 2026-10-14
- **Autor:** agente core
- **Modelo:** GPT-6; esforço não informado
- **Escopo:** API pública de miupy-core para RF-22, RF-24, RF-25, RF-27 e reprocessamento de metadados.

## O que foi feito

- Acrescentei a migração `0003_dna_shuffle_images.sql`, com `artists.image_credit` e `settings.shuffle_mode` (`off`, `random`, `smart`). O teste parte de um banco v2 com artista, faixa, favorita, reprodução, perfil e configuração existentes e verifica que esses dados sobrevivem à migração.
- Favoritas sem reprodução agora entram na pontuação por faixa, perfil de gosto, sementes e recomendações.
- Adicionei pontuação por faixa, DNA com janelas consecutivas, Fisher–Yates determinístico e ordenação smart ponderada com semente explícita. O modo salvo fica separado do tipo `Settings` do IPC.
- `Playlist` inclui `mosaic_paths`; o mosaico considera os quatro artistas mais frequentes antes de retirar os que não têm imagem. Adicionei setters para capa de playlist, avatar e foto/crédito do artista, devolvendo o caminho antigo para remoção pela casca.
- Adicionei a lista de faixas para reprocessamento e confirmei que `TrackPatch` permite corrigir título, artista e álbum.
- Atualizei a documentação da fórmula de recomendação e do banco.

## Decisões

- O modelo `Playlist` do core traz `mosaic_paths`, calculado em `get_playlist` e `list_playlists`; a casca pode consumir o campo diretamente.
- Pontuação e estatísticas de DNA compartilham `taste_data_window`, para manter a fórmula única. A janela atual inclui `[agora − janela, agora)` e a anterior `[agora − 2 × janela, agora − janela)`; eventos abertos ficam fora. Favoritas entram nas duas janelas sem depender de reprodução.
- Os sorteios são funções puras: a casca fornece os dados e a semente. Não foi adicionada dependência de PRNG.
- Não alterei `ipc.md`. A matriz gerada e `arquitetura.json` ficaram fora da lista de arquivos permitidos para esta tarefa.

## API pública nova

```rust
pub fn random_order(track_ids: &[String], seed: u64) -> Vec<String>
pub fn smart_order(current: &SmartTrack, remaining: &[SmartTrack], seed: u64) -> Vec<String>

impl Database {
    pub fn track_scores(&self, track_ids: Option<&[String]>) -> Result<Vec<TrackScore>>
    pub fn smart_tracks(&self, track_ids: &[String]) -> Result<Vec<SmartTrack>>
    pub fn dna(&self, window_days: Option<u32>, now: i64) -> Result<Dna>
    pub fn get_shuffle_mode(&self) -> Result<ShuffleMode>
    pub fn set_shuffle_mode(&self, mode: ShuffleMode) -> Result<ShuffleMode>
    pub fn playlist_mosaic_paths(&self, playlist_id: &str) -> Result<Vec<String>>
    pub fn set_playlist_cover(
        &self, playlist_id: &str, cover_path: Option<&str>
    ) -> Result<Option<String>>
    pub fn set_profile_avatar(&self, avatar_path: Option<&str>) -> Result<Option<String>>
    pub fn set_artist_image(
        &self, artist_id: &str, image_path: Option<&str>, image_credit: Option<&str>
    ) -> Result<Option<String>>
    pub fn tracks_for_metadata_refresh(&self, only_missing: bool) -> Result<Vec<Track>>
}
```

`Playlist` agora serializa `mosaic_paths`; `ArtistInfo` agora serializa `image_credit`. `Dna`, `DnaTrend`, `TrackScore`, `SmartTrack` e `ShuffleMode` são exportados pelo crate. `TrackPatch` já permitia editar título, artista e álbum, portanto sua assinatura não mudou.

## Verificação (comandos rodados e resultado real)

### Verificado rodando

- `cargo fmt --all`: terminou sem erros.
- `git diff --check`: terminou sem erros.
- `nice -n 10 cargo clippy --workspace --all-targets`: terminou sem warnings.
- `nice -n 10 cargo test --workspace`: passou. Resultado agregado: **233 aprovados, 0 falhas, 10 ignorados**; inclui 74 testes da casca, 26 de áudio, 76 do core, 56 de mídia e 1 doctest.
- O teste de migração v2 com dados e os novos testes de favoritas, pontuação, DNA, shuffle, mosaico, imagem e reprocessamento foram executados dentro da suíte do core.

### Só documentado ou escrito

- Não rodei `npm run docs:matriz`: o script grava `docs/rastreabilidade/matriz.md` e `matriz.json`, fora da lista de arquivos permitidos.
- Não rodei a verificação contra rede real; esta tarefa do core não adicionou comportamento dependente de rede.

## Pendências e problemas conhecidos

- `docs/arquitetura/arquitetura.json` não foi atualizado porque está fora do escopo de arquivos permitido, embora o core tenha ganhado o módulo `shuffle`.
- A casca deve persistir e fornecer a semente do sorteio, usar `smart_tracks` para montar os dados, apagar os caminhos antigos devolvidos pelos setters e encaminhar `only_missing` para `tracks_for_metadata_refresh`.

## Próximos passos

- A casca pode ligar os comandos de IPC às assinaturas listadas acima. O contrato frontend ⇄ Rust permaneceu inalterado.
