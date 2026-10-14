# Recomendação local (RF-19)

O núcleo não consulta a rede. A casca pede sementes, busca vídeos, grava candidatas e consulta o
ranking. Todas as contas usam o histórico local de eventos **encerrados** e as favoritas. Uma
playlist considera o histórico inteiro das faixas que estão nela agora, inclusive reproduções
feitas fora da playlist.

## Pontuação da faixa

Para cada evento encerrado, `f = min(tempo_ouvido / duração, 1)`; se a duração é desconhecida,
`f = 0`. A idade é medida entre o início do evento e o relógio do banco. O peso de recência é
`2 ^ (-idade_em_dias / 90)`, com idade mínima zero. A contribuição do evento é:

```text
recência × (0,45 × f + 0,80 × conclusão − pulo × (1,10 − 0,70 × f))
```

`conclusão` e `pulo` valem 0 ou 1. Um pulo aos 5% da faixa custa 1,065; aos 90%, 0,47. Isso
evita tratar uma desistência quase no fim como rejeição forte. A faixa soma os eventos, mais
`0,65` se é favorita e `0,15 × ln(1 + número_de_conclusões)` pela repetição. O bônus de repetição
é logarítmico para não deixar uma única faixa dominar só porque foi repetida em loop. Os pesos
são constantes nomeadas em `recommendations.rs`.

O perfil soma as pontuações das faixas por artista e gênero. Artistas com saldo negativo não
entram nos `topArtists`, mas o saldo negativo reduz candidatas desse artista. `skipRate` é
`eventos encerrados e pulados / eventos encerrados`; `totalListenedMs` soma todo o tempo ouvido
desses eventos. Sem eventos encerrados, perfil vazio e lista de recomendações vazia.

## Sementes e candidatas

`recommendation_seeds` devolve faixas com `sourceId`, artistas e gêneros com saldo positivo,
em ordem de pontuação, respeitando limites independentes (até 100 de cada). A casca usa as
sementes para buscar vídeos. `upsert_recommendation_candidate` guarda um vídeo por `sourceId`,
normaliza gêneros e registra tipo, rótulo e, para faixa parecida, `seedTrackId`. Uma candidata
dispensada fica como lápide permanente e nenhum upsert a revive. Uma faixa que entra na discoteca
é removida da tabela por trigger; lápides dispensadas permanecem. A consulta também exclui qualquer
`sourceId` já presente em `tracks`, inclusive registros importados por outro caminho.

## Ranking

Cada candidata recebe uma base `0,15`, mais `0,45 × saldo_normalizado_do_artista`,
`0,25 × maior_saldo_normalizado_de_gênero` e `0,35 × saldo_normalizado_da_semente`.
O denominador de cada saldo é o maior saldo positivo do mesmo tipo, com piso 1; valores negativos
continuam negativos. O resultado é limitado a `0..1`. A candidata só entra se tiver alguma
afinidade positiva. Os motivos (`reasons`) vêm das contribuições positivas, em ordem de força,
com até três entradas. O ranking usa desempate estável por `sourceId` e limita a três vídeos por
artista. `limit` é 20 por padrão e no máximo 100; a diversidade pode devolver menos que o limite.

O modelo deliberadamente não usa similaridade textual de título nem dados externos no cálculo.
Gênero desconhecido é `[]`; isso não impede recomendação por artista ou por semente de faixa.

## Favoritas sem reprodução

Uma faixa favoritada entra no gosto mesmo sem nenhum evento: soma o bônus de favorita (`0,65`) à
faixa e, por ela, ao artista e aos gêneros. Isso vale para `stats_taste`, `stats_dna`, sementes e
ranking. Um perfil só com favoritas não é vazio.

## *Smart random* (RF-24)

O núcleo recebe as faixas restantes da fila e devolve uma ordem. É uma função pura sobre as
pontuações, artistas e gêneros, com semente de sorteio explícita para os testes. A cada passo,
sorteia uma faixa entre as que faltam com peso:

```text
peso = 0,10 + 0,60 × pontuação_normalizada + 0,30 × proximidade
```

- `pontuação_normalizada = max(pontuação, 0) / max(maior pontuação positiva da fila, 1)`.
- `proximidade` com a faixa escolhida no passo anterior (no primeiro passo, a faixa atual):
  `1` se é do mesmo artista, `0,5` se divide pelo menos um gênero, `0` caso contrário. Se as duas
  últimas escolhidas já são do mesmo artista, a proximidade por artista vale `0` para evitar uma
  sequência longa de um só artista.
- A base `0,10` garante que toda faixa sai em algum momento.

## DNA (RF-25)

`stats_dna` aplica a mesma pontuação de faixa só aos eventos de cada janela (a atual e a
anterior, de `windowDays` cada), sem o decaimento de recência dentro da janela, e soma por artista
e por gênero. O bônus de favorita entra nas duas janelas. `rising` e `falling` ordenam a diferença
entre as janelas, ignorando diferenças abaixo de `0,1`.
