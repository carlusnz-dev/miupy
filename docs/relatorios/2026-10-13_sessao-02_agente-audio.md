# Áudio real e streaming HTTP

- **Data:** 2026-10-13
- **Autor:** agente audio
- **Modelo:** gpt-6-sol (esforço high), via `codex exec`
- **Escopo:** ampliar `miupy-audio` para M4A/AAC, MP3, eventos de erro e streaming RF-18.

## O que foi feito

- Criei fixtures com `ffmpeg`: M4A/AAC, MP3, M4A fragmentado e M4A sem duração nos átomos MP4. O arquivo sem duração foi produzido zerando `mvhd`, `mdhd` e `tkhd` depois da codificação. Acrescentei uma fixture fragmentada de cinco segundos para simular queda de rede depois do início da reprodução.
- A duração ausente é calculada contando as amostras decodificadas em segundo plano. Os testes conferem cerca de 2 s para todos os formatos e a atualização de `PlaybackState` depois de `load`.
- `RodioBackend` emite `AudioEvent::Error` na falha de abertura/decodificação, na interrupção precoce da faixa, na falha do dispositivo e na falha da rede. Após o erro, o estado volta a `Stopped` e aceita novo `load`. A abertura do dispositivo passou a ocorrer depois da abertura e decodificação inicial do arquivo; isso preserva o erro específico de arquivo mesmo sem saída de áudio.
- Acrescentei streaming HTTP com cabeçalhos, duração antecipada, cache em memória, Range, abertura fora da thread chamadora, `Loading`, seek, progresso, fim natural e recuperação de falha. `play` chamado em `Loading` inicia quando o decoder fica pronto. O backend nulo simula `load_url` com a duração recebida.
- Atualizei o README da crate.

### API pública nova

```rust
pub struct StreamOptions {
    pub headers: Vec<(String, String)>,
    pub duration_ms: u64,
}
pub trait AudioBackend: Send + Sync {
    fn load_url(&self, url: &str, options: StreamOptions) -> Result<()>;
    // demais métodos existentes
}
pub struct PlaybackState {
    pub url: Option<String>,
    // demais campos existentes
}
pub enum AudioError {
    Network { url: String, message: String },
    // demais variantes existentes
}
```

`load_url` retorna antes da primeira requisição HTTP terminar. Falhas posteriores chegam por `AudioEvent::Error`. A adição do método obrigatório quebra implementações externas de `AudioBackend`; a adição de `url` quebra literais completos e destructuring exaustivo de `PlaybackState`. A nova variante `Network` quebra matches exaustivos de `AudioError`. A casca terá de chamar `load_url` e mapear `state().url`/erros; esse trabalho está fora desta frente.

## Decisões

- Usei `stream-download` 0.24.4 com `HttpStream<reqwest::Client>` e `MemoryStorageProvider`. Ele já fornece `Read + Seek`, cache e requisições Range; um leitor HTTP próprio exigiria sincronizar download, seek e cancelamento. Abertura e runtime Tokio ficam numa thread própria. Um watchdog de 5 s converte estagnação de download em erro.
- O cache em memória pode crescer até o tamanho da faixa. Para o áudio M4A curto e de bitrate baixo do produto, isso simplifica o motor; uma política de cache em disco pode ser considerada se faixas longas causarem pressão de memória.
- A detecção de decodificação interrompida usa erro do leitor e, no fim do player, compara a posição alcançada com a duração conhecida (tolerância de 250 ms). Pacotes danificados que o próprio Symphonia ignora e que não encurtam a faixa podem continuar sem evento de erro.
- Não alterei o contrato IPC. `Loading` já existe nele. O backend recebe a URL resolvida da casca e não escolhe como o yt-dlp será distribuído.

## Verificação (comandos rodados e resultado real)

| Comando | Resultado |
|---|---|
| `cargo fmt --all` | Concluiu sem erro. |
| `cargo clippy -p miupy-audio --all-targets -- -D warnings` | Passou, 0 warnings. |
| `cargo test -p miupy-audio -- --nocapture` | 25 testes passaram, 0 falhas, 0 ignorados; doc-tests: 0. |
| `git diff --check` | Passou, sem erros de whitespace nos arquivos rastreados. |
| `git restore --source=HEAD -- Cargo.lock` | Falhou: `Unable to create .../index.lock: Read-only file system`. O lockfile raiz permanece modificado por `cargo add`. |
| `git add -A && git commit -m 'Implementa áudio real e streaming HTTP'` | Falhou no `git add -A` pelo mesmo `index.lock: Read-only file system`; o `git commit` não foi executado por causa do `&&`. Nenhum commit foi criado. |

Os testes de reprodução abriram o dispositivo de saída neste ambiente (nenhuma mensagem `PULADO:` apareceu), avançaram o player e receberam `TrackEnded` para M4A, M4A sem duração, fMP4, MP3 e streaming local. Os testes de arquivo rodaram com volume silenciado. Não houve escuta humana para atestar som audível. Decodificação, duração, cabeçalhos, Range e queda de conexão também foram verificados diretamente sem depender da saída. A retirada física do dispositivo não foi feita; o teste injeta o erro do callback do rodio e comprova `Error` e recuperação. O servidor local corta uma resposta após o início da reprodução e comprova `Error`, ausência de `TrackEnded` e novo `load` funcional.

## Pendências e problemas conhecidos

- No servidor local, o teste de início antecipado de fMP4 confirma que o decoder fica pronto antes dos últimos 1 KiB da resposta. O decoder de MP3 precisou de mais dados na inicialização; a antecipação do início de MP3 não foi demonstrada. O seek com Range foi verificado para MP3 e fMP4.
- O `Cargo.lock` raiz foi atualizado por `cargo add`, mas está fora do escopo de arquivos desta frente. A tentativa de restaurá-lo pelo Git falhou por permissão no diretório de metadados; a orquestração precisa decidir como integrar o lockfile.
- O teste não cobriu perda física do dispositivo nem rede externa. O servidor HTTP é local e reproduz cabeçalhos, Range e queda de conexão.

## Próximos passos

1. Integrar `load_url` na casca, resolvendo a URL na outra frente e repassando cabeçalhos e duração.
2. Conferir o comportamento com URLs reais do yt-dlp e com um dispositivo de áudio físico.
3. Resolver o `Cargo.lock` e o commit na orquestração se o diretório Git continuar somente leitura.

## Nota da orquestração (revisão antes do merge)

O commit foi feito pela orquestração (o sandbox do agente não escreve no diretório git). O que
está acima foi verificado só contra um servidor `http://` local com áudio de 2 a 5 s. A
orquestração escreveu um teste contra a rede real (`real_stream_plays_seeks_and_keeps_playing`,
ignorado por padrão, URL em `MIUPY_STREAM_URL`) e ele revelou quatro defeitos, todos corrigidos
antes do merge:

1. **HTTPS não funcionava.** `stream-download` foi adicionado só com a feature `reqwest`, sem
   TLS; toda URL do YouTube falhava na hora. Trocado para `reqwest-rustls`.
2. **A faixa morria cerca de 5 s depois de o download terminar.** O vigia de rede media o
   andamento do download; uma faixa que chegou inteira para de baixar e era dada como "rede
   interrompida". O vigia passou a medir quanto tempo o decoder espera por bytes
   (`STREAM_STALL_TIMEOUT`, 8 s). O cliente HTTP tinha prazo total de 5 s para o corpo inteiro;
   trocado por prazo de conexão e de leitura.
3. **Seek para frente levava cerca de 18 s** (o decoder era aberto sem busca por bytes e
   decodificava tudo até o ponto). Agora abre com `with_byte_len` + `with_seekable(true)`. A
   contrapartida: a faixa só fica pronta com o arquivo inteiro baixado, e o teste que afirmava
   "começa antes do download completo" foi reescrito para o comportamento real. A abertura tem
   prazo próprio (`STREAM_OPEN_TIMEOUT`, 30 s).
4. **Buffer underrun da saída de som era tratado como falha fatal** e descarregava a faixa.
   Underrun e overrun passaram a ser ignorados.

Resultado real depois das correções, rodado pela orquestração: `cargo clippy -p miupy-audio
--all-targets` com 0 warnings; `cargo test -p miupy-audio` com 26 passando e 1 ignorado; o teste
de rede real passou em 6 de 6 execuções com três vídeos diferentes (primeiro áudio entre 1,5 e
3,5 s; 12 s de reprodução sem erro; seek para 60 s em menos de 40 ms). A reprodução foi feita
com o backend mudo: ninguém ouviu o som.
