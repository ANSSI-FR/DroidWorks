<template>
  <div class="d-flex justify-content-between align-items-center">
    <div class="pe-3 fst-italic">
      {{ flag.name }}
    </div>
    <select
      class="form-select"
      :disabled="locked"
      @change="changeFlagValue($event)"
    >
      <option
        value="true"
        :selected="flag.value === true"
      >
        {{ $t("yes") }}
      </option>
      <option
        value="false"
        :selected="flag.value === false"
      >
        {{ $t("no") }}
      </option>
      <option
        value="default"
        :selected="flag.value === null"
      >
        {{ $t("default") }}
      </option>
    </select>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import { useStore } from "@/store";

export default defineComponent({
  name: "FlagItem",
  props: {
    flag: {
      type: Object,
      required: true,
    },
  },
  setup() {
    const store = useStore();
    const locked = computed(() => store.getters["global/isLocked"]);
    const setApplicationFlagValue = (
      name: string,
      value: boolean | undefined
    ): void => {
      store.commit("manifest/setApplicationFlagValue", { name, value });
    };

    return { locked, setApplicationFlagValue };
  },
  methods: {
    changeFlagValue(event: Event) {
      if (!this.flag) {
        return;
      }
      const newValue = (event.target as HTMLInputElement).value;
      if (newValue === "true") {
        this.setApplicationFlagValue(this.flag.name, true);
      } else if (newValue === "false") {
        this.setApplicationFlagValue(this.flag.name, false);
      } else {
        this.setApplicationFlagValue(this.flag.name, undefined);
      }
    },
  },
});
</script>
