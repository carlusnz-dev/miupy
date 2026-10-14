// @req RNF-10
import { flipTransform, registerSharedEntry, SharedSnapshot, type SharedRect } from './motion';

type Entry = { node: HTMLElement; key: string; generation: number };
const byKey = new Map<string, Set<Entry>>();
const before = new SharedSnapshot<SharedRect>();
let generation = 0;

function rect(node: HTMLElement): SharedRect {
  const box = node.getBoundingClientRect();
  return { left: box.left, top: box.top, width: box.width, height: box.height };
}

export function captureSharedLayout(preferPersistentPlayer = false) {
  generation++;
  const snapshot = new Map<string, SharedRect>();
  for (const [key, group] of byKey) for (const { node } of group) if (node.isConnected) snapshot.set(key, rect(node));
  before.replace(snapshot);
  const capturedGeneration = generation;
  requestAnimationFrame(() => {
    if (capturedGeneration !== generation) return;
    for (const [key, group] of byKey) {
      const connected = [...group].filter(({ node }) => node.isConnected && node.getClientRects().length > 0);
      const added = connected.filter((entry) => entry.generation === capturedGeneration);
      const persistentPlayer = preferPersistentPlayer
        ? connected.find(({ node }) => !node.closest('#conteudo'))
        : undefined;
      const target = (added.at(-1) ?? persistentPlayer ?? connected.at(-1))?.node;
      const from = before.take(key);
      if (!target || !from) continue;
      const to = rect(target);
      target.getAnimations().forEach((animation) => animation.cancel());
      if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
        target.animate([{ opacity: 0.35 }, { opacity: 1 }], { duration: 100, easing: 'ease-out' });
      } else {
        const { x, y, scaleX, scaleY } = flipTransform(from, to);
        target.animate([
          { transformOrigin: 'top left', transform: `translate3d(${x}px, ${y}px, 0) scale(${scaleX}, ${scaleY})` },
          { transformOrigin: 'top left', transform: 'translate3d(0, 0, 0) scale(1, 1)' },
        ], { duration: 520, easing: 'cubic-bezier(.22, 1.45, .36, 1)', composite: 'replace' });
      }
    }
  });
}

export function useShared(node: HTMLElement, key: string) {
  if (!key) return { destroy() {} };
  const entry = { node, key, generation };
  const unregister = registerSharedEntry(byKey, key, entry);
  return {
    destroy() { unregister(); },
  };
}

// @test RNF-10
export function sharedEntryCount() { return [...byKey.values()].reduce((count, group) => count + group.size, 0); }
