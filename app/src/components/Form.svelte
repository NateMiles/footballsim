<!-- Recent results as W/D/L chips. -->
<script lang="ts">
  let { form }: { form: string[] } = $props();
</script>

<span class="form">
  {#each form as r, i (i)}
    <span class="r {r}">{r}</span>
  {/each}
</span>

<style>
  .form {
    display: inline-flex;
    gap: 3px;
  }

  .r {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 700;
    color: #071209;
  }

  .W {
    background: var(--good);
  }

  .D {
    background: #b9c3bd;
  }

  .L {
    background: var(--bad);
  }
</style>
