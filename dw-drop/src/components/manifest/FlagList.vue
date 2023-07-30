<template>
  <div class="flex-fill">
    <div class="d-flex flex-wrap justify-content-evenly">
      <div
        class="card text-white bg-secondary mb-3"
        style="min-width: 25rem; max-width: 25rem"
      >
        <div class="card-header">
          {{ $t("tab.manifest") }}
        </div>
        <div class="card-body">
          <div class="d-flex justify-content-between">
            <h6 class="card-title fst-italic">
              Package
            </h6>
            <div style="text-align: right">
              {{ infos.package }}
            </div>
          </div>
          <div class="d-flex justify-content-between">
            <h6 class="card-title fst-italic">
              Version code
            </h6>
            <div style="text-align: right">
              {{ infos.versionCode }}
            </div>
          </div>
          <div class="d-flex justify-content-between">
            <h6 class="card-title fst-italic">
              Version name
            </h6>
            <div style="text-align: right">
              {{ infos.versionName }}
            </div>
          </div>
          <div class="d-flex justify-content-between">
            <h6 class="card-title fst-italic">
              Minimum SDK version
            </h6>
            <div style="text-align: right">
              {{ infos.minSdkVersion }}
            </div>
          </div>
        </div>
      </div>
      <div
        class="card text-dark bg-light mb-3"
        style="min-width: 30rem; max-width: 30rem"
      >
        <div class="card-header">
          {{ $t("tab.application") }}
        </div>
        <div class="card-body">
          <FlagItem
            v-for="flag in items"
            :key="flag.name"
            :flag="flag"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from "vue";
import FlagItem from "@/components/manifest/FlagItem.vue";
import Flag from "@/typings/manifest/flag";

export default defineComponent({
  name: "FlagList",
  components: {
    FlagItem,
  },
  props: {
    infos: {
      type: Object,
      required: true,
    },
    items: Array as PropType<Flag[]>,
  },
});
</script>
