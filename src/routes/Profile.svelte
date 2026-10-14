<script lang="ts">
  // Perfil com validação. @req RF-16
  import { errorMessage, type Profile } from '../lib/ipc';
  import { profile } from '../lib/stores/profile.svelte';
  import { notify } from '../lib/stores/toast.svelte';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import State from '../lib/components/State.svelte';

  let form = $state<Profile | null>(null);
  let errors = $state<Partial<Record<'displayName' | 'username' | 'email' | 'phone', string>>>({});
  let saving = $state(false);
  let formError = $state('');
  let loadError = $state<string | null>(null);

  async function load() {
    try { if (!profile.data) await profile.load(); form = { ...profile.data! }; loadError = null; }
    catch (e) { loadError = errorMessage(e); }
  }
  $effect(() => { void load(); });

  function validate(p: Profile) {
    const e: typeof errors = {};
    const name = p.displayName.trim();
    if (name.length < 2) e.displayName = 'Informe seu nome (mínimo 2 caracteres).';
    else if (name.length > 80) e.displayName = 'Nome longo demais (máximo 80).';
    if (!/^[a-z0-9_.]{3,30}$/i.test(p.username.trim())) e.username = 'Use 3 a 30 caracteres: letras, números, ponto ou sublinhado.';
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(p.email.trim())) e.email = 'E-mail inválido. Exemplo: nome@exemplo.com';
    const phone = p.phone.trim();
    if (phone && !/^\+?[\d\s().-]+$/.test(phone)) e.phone = 'Use apenas números, espaços, +, ( ) e -.';
    else if (phone && (phone.replace(/\D/g, '').length < 8 || phone.replace(/\D/g, '').length > 15)) e.phone = 'O telefone deve ter de 8 a 15 dígitos.';
    return e;
  }

  async function submit(ev: SubmitEvent) {
    ev.preventDefault();
    if (!form) return;
    errors = validate(form);
    formError = '';
    if (Object.keys(errors).length) {
      document.getElementById(`pf-${Object.keys(errors)[0]}`)?.focus();
      return;
    }
    saving = true;
    try {
      const saved = await profile.save({ ...form, displayName: form.displayName.trim(), username: form.username.trim(), email: form.email.trim(), phone: form.phone.trim() });
      form = { ...saved };
      notify('Perfil salvo.');
    } catch (e) { formError = errorMessage(e); } finally { saving = false; }
  }

  const fields = [
    { key: 'displayName', label: 'Nome', type: 'text', auto: 'name' },
    { key: 'username', label: 'Nome de usuário', type: 'text', auto: 'username' },
    { key: 'email', label: 'E-mail', type: 'email', auto: 'email' },
    { key: 'phone', label: 'Telefone', type: 'tel', auto: 'tel' },
  ] as const;
</script>

<PageHeader title="Perfil" subtitle="Seus dados ficam apenas neste dispositivo." />

{#if loadError}
  <State kind="error" text={loadError} onretry={load} />
{:else if !form}
  <State kind="loading" />
{:else}
  <div class="grid max-w-3xl gap-6 md:grid-cols-[auto_1fr]">
    <div class="flex flex-col items-center gap-2">
      <div class="grid h-28 w-28 place-items-center rounded-full text-4xl font-black" style="background:var(--grad-lava);color:var(--lava-ink)" role="img" aria-label="Avatar com as iniciais {profile.initials}">{profile.initials}</div>
      <p class="text-sm font-medium">{form.displayName || '—'}</p>
      <p class="text-xs text-muted">@{form.username || 'usuario'}</p>
    </div>
    <form class="card flex flex-col gap-4 p-5" onsubmit={submit} novalidate>
      {#each fields as f}
        <div>
          <label class="mb-1 block text-sm text-muted" for="pf-{f.key}">{f.label}</label>
          <input id="pf-{f.key}" class="field" type={f.type} autocomplete={f.auto} bind:value={form[f.key]}
            aria-invalid={errors[f.key] ? 'true' : undefined} aria-describedby={errors[f.key] ? `pf-${f.key}-err` : undefined} />
          {#if errors[f.key]}<p id="pf-{f.key}-err" class="mt-1 text-sm text-danger" role="alert">{errors[f.key]}</p>{/if}
        </div>
      {/each}
      {#if formError}<p class="text-sm text-danger" role="alert">{formError}</p>{/if}
      <div class="flex justify-end gap-2">
        <button type="button" class="btn" onclick={() => { form = { ...profile.data! }; errors = {}; }}>Desfazer</button>
        <button class="btn btn-primary" disabled={saving}>{saving ? 'Salvando…' : 'Salvar'}</button>
      </div>
    </form>
  </div>
{/if}
