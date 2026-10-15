// @test RF-05
import { describe, expect, it } from 'vitest';
import { fmtAddedAgo } from './age';

const DAY = 86_400_000;
const now = 1_800_000_000_000;
const ago = (days: number) => fmtAddedAgo(now - days * DAY, now);

describe('fmtAddedAgo', () => {
  it('usa dias, meses e anos conforme o tempo', () => {
    expect(ago(0)).toBe('hoje');
    expect(ago(1)).toBe('há 1 dia');
    expect(ago(3)).toBe('há 3 dias');
    expect(ago(29)).toBe('há 29 dias');
    expect(ago(30)).toBe('há 1 mês');
    expect(ago(75)).toBe('há 2 meses');
    expect(ago(364)).toBe('há 12 meses');
    expect(ago(365)).toBe('há 1 ano');
    expect(ago(800)).toBe('há 2 anos');
  });
  it('trata data no futuro como hoje', () => {
    expect(fmtAddedAgo(now + DAY, now)).toBe('hoje');
  });
});
