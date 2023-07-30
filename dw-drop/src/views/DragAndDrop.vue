<template>
  <div
    id="draganddrop"
    class="position-fixed d-flex flex-column align-items-center justify-content-between"
    :class="{
      'bg-white': !active,
      'bg-primary': active,
      'text-black': !active,
      'text-white': active,
    }"
    style="z-index: -1"
  >
    <span />
    <i class="huge bi bi-download" />
    <span class="p-1 w-100 text-center">
      {{ $t("draganddrop1") }}<br>{{ $t("draganddrop2") }}
    </span>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import Status from "@/typings/status";
import { event } from "@tauri-apps/api";
import { useStore } from "@/store";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "DragAndDrop",
  setup() {
    const store = useStore();
    const status = computed(() => store.getters["global/getStatus"]);
    const openApplication = async (filename: string): Promise<void> => {
      return store.dispatch("global/openApplication", filename);
    };

    const router = useRouter();

    return { status, openApplication, router };
  },
  data() {
    return {
      Status,
      active: false,
    };
  },
  mounted() {
    event.listen("tauri://file-drop-hover", (event) => {
      console.log(event);
      this.active = true;
    });
    event.listen("tauri://file-drop-cancelled", (event) => {
      console.log(event);
      this.active = false;
    });
    event.listen<string[]>("tauri://file-drop", (event) => {
      console.log(event);
      this.active = false;
      this.openApplication(unescape(event.payload[0])).then(() => {
        if (this.status == Status.STATUS_LOADED) {
          this.router.push({ name: "Manifest" });
        }
      });
    });
  },
});
</script>

<style scoped>
#draganddrop {
  padding: 10px;
  width: 600px;
  height: 400px;
  right: calc(50% - (600px / 2));
  top: calc(((100% - 106px) - (400px / 2)) / 2);
  border: dashed 4px #bbbbbb;
  border-radius: 10px;
}

.huge {
  font-size: 90pt;
  opacity: 0.75;
}
</style>
