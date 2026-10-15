// @test RF-05
import { describe, expect, it } from 'vitest';
import { assetCrossOrigin } from './asset';

describe('assetCrossOrigin', () => {
  it('pede CORS para imagens locais do protocolo asset (Linux e Windows)', () => {
    expect(assetCrossOrigin('asset://localhost/%2Fhome%2Fx%2Fcapa.jpg')).toBe('anonymous');
    expect(assetCrossOrigin('http://asset.localhost/C%3A%2Fx%2Fcapa.jpg')).toBe('anonymous');
  });
  it('deixa sem atributo as miniaturas remotas e a ausência de imagem', () => {
    expect(assetCrossOrigin('https://i.ytimg.com/vi/x/hqdefault.jpg')).toBeUndefined();
    expect(assetCrossOrigin(null)).toBeUndefined();
    expect(assetCrossOrigin('data:image/png;base64,AA')).toBeUndefined();
  });
});
