// @req RF-21 RF-22
import type { ImageUpload } from '../ipc/types';
const formats = new Set(['image/jpeg', 'image/png', 'image/webp']);
export async function readImageUpload(file: File): Promise<ImageUpload> {
  if (!formats.has(file.type)) throw new Error('Escolha uma imagem JPEG, PNG ou WebP.');
  if (file.size > 10 * 1024 * 1024) throw new Error('A imagem deve ter no máximo 10 MB.');
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(new Error('Não foi possível ler a imagem.'));
    reader.readAsDataURL(file);
  });
  const dataBase64 = dataUrl.split(',')[1];
  if (!dataBase64) throw new Error('Imagem inválida.');
  return { dataBase64, mimeType: file.type };
}
export function previewImage(image: ImageUpload) { return `data:${image.mimeType};base64,${image.dataBase64}`; }
