# Rastreabilidade

- [`matriz.md`](matriz.md) e [`matriz.json`](matriz.json): gerados por
  `scripts/rastreabilidade.mjs`. Não edite à mão.

## Como funciona

O script lê os IDs de `docs/requisitos/requisitos.md` e procura, no código, comentários com
`@req <IDs>` (implementação) e `@test <IDs>` (teste). Cada requisito recebe um status:

| Status | Significado |
|---|---|
| pendente | Nenhuma anotação. |
| implementado | Tem `@req`, não tem `@test`. |
| verificado | Tem `@req` e `@test`. |
| só teste | Tem `@test` sem `@req`. |

"Verificado" quer dizer que existe um teste anotado, não que o teste passa: isso quem diz é o
`cargo test`.

## Comandos

```sh
npm run docs:matriz                       # regenera a matriz
node scripts/rastreabilidade.mjs --check  # falha se houver ID anotado que não existe
```
