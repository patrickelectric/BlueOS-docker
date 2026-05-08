<template>
  <div class="d-flex flex-column align-center" style="min-width: 64px;">
    <div class="swatch rounded" :style="{ background: color }" />
    <div class="text-caption mt-1">
      {{ label }}
    </div>
    <div class="text-caption text--secondary text-monospace">
      {{ color }}
    </div>
  </div>
</template>

<script lang="ts">
import Vue from 'vue'

export default Vue.extend({
  name: 'ColorSwatch',
  props: {
    color: {
      type: String,
      required: true,
    },
    label: {
      type: String,
      required: true,
    },
  },
})
</script>

<style scoped>
.swatch {
  width: 56px;
  height: 32px;
  border: 1px solid rgba(0, 0, 0, 0.12);
}
</style>
