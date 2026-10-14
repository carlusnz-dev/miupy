// @req RF-16
import { api, type ImageUpload, type Profile } from '../ipc';

class ProfileStore {
  data = $state<Profile | null>(null);
  async load() { this.data = await api.profileGet(); }
  async save(p: Profile) { this.data = await api.profileUpdate({ profile: p }); return this.data; }
  async setAvatar(image: ImageUpload | null) { this.data = await api.profileSetAvatar({ image }); return this.data; } // @req RF-21
  get initials() {
    const n = (this.data?.displayName || this.data?.username || '?').trim();
    return n.split(/\s+/).slice(0, 2).map((s) => s[0]?.toUpperCase()).join('');
  }
}
export const profile = new ProfileStore();
