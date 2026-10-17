// @req RF-46 RNF-08
export function beginDrag(start: number, time: number) {
  let last = start;
  let lastTime = time;
  let distance = 0;
  let velocity = 0;
  let cancelled = false;
  return {
    move(position: number, now: number) {
      if (cancelled) return 0;
      distance = position - start;
      if (now > lastTime && position !== last) {
        velocity = (position - last) / (now - lastTime);
        last = position;
        lastTime = now;
      }
      return distance;
    },
    finish(size: number, direction: -1 | 1, fraction: number, now = lastTime) {
      return !cancelled && size > 0 && distance * direction > 0 &&
        (distance * direction >= size * fraction || (now - lastTime <= 80 && velocity * direction >= 0.65));
    },
    cancel() { cancelled = true; distance = 0; velocity = 0; },
  };
}
