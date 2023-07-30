<template>
  <div
    id="spinner-container"
    class="position-fixed bottom-50 d-flex align-items-center justify-content-center"
    style="z-index: 3000"
  >
    <div
      class="spinner-border"
      role="status"
    >
      <span class="sr-only" />
    </div>
    <span class="ps-3 fw-bolder">{{ message }}...</span>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";

export default defineComponent({
  name: "LoadingSpinner",
  props: {
    message: String,
  },
});
</script>

<style scoped>
#spinner-container {
  width: 500px;
  right: calc(50% - (500px / 2));
}
</style>
