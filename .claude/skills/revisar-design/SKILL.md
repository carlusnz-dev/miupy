---
name: revisar-design
description: Revisão de design e UX de um diff do miupy lida no código (sem prints). Use ao revisar PR que toca em src/ ou antes de entregar uma mudança visual.
---

# Revisar design (pelo código)

Barato: leia o diff de `src/` e confira a lista. Só abra o navegador (`revisar-playwright`) para o
que o código não responde.

```sh
git diff develop...HEAD --stat -- src && git diff develop...HEAD -- src
```

## Lista

1. **Tokens:** nenhuma cor/raio/sombra literal nova (`#`, `rgb(`, `box-shadow`) fora de
   `src/app.css`; usa `var(--…)` do magma ou as classes Tailwind ligadas a ele (`bg-s1`,
   `text-lava-text`…). Sobreposição `bg-black/50` em cima de capa é aceita (já é o padrão).
2. **Componentes do app:** nenhum `<select>`, checkbox ou botão cru onde existe componente do
   tema; componente novo vai para `src/lib/components/` e é reaproveitado.
3. **Estados:** carregando (esqueleto no formato), vazio (convida a agir), erro (diz o que fazer)
   em toda lista ou tela com dados.
4. **Teclado e foco:** tudo clicável é `button`/`a`; Escape fecha popups e o foco volta a quem
   abriu; foco visível; `aria-expanded`/`aria-pressed`/`aria-label` corretos.
5. **Movimento:** consistente entre os itens da lista; respeita `prefers-reduced-motion`; nenhum
   trabalho caro por quadro (RNF-13).
6. **Layout:** alinhamento coerente com a tela vizinha; nada desalinhado (ex.: volume fora do eixo
   dos controles); quebra de linha controlada.
7. **Texto:** pt-BR, sentença, verbo de resultado, mesmo nome no fluxo; números e tempos
   formatados como no resto do app.
8. **Seleção de texto:** a interface não seleciona texto (só inputs).

Resultado: lista de achados com `arquivo:linha`, o que o usuário veria e a correção. Diga o que
não dá para saber pelo código (vai para `revisar-playwright` ou para o autor no app real).
