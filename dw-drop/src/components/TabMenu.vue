<template>
  <ul class="nav nav-tabs pt-2 bg-white">
    <li class="nav-item">
      <router-link
        to="/manifest"
        class="nav-link"
        :class="{
          active: currentRoute == 'Manifest',
          disabled: status != Status.STATUS_LOADED,
        }"
      >
        <i class="bi bi-map">&nbsp;</i>
        {{ $t("tab.manifest") }}
        <span v-if="manifestIsModified">*</span>
      </router-link>
    </li>
    <li class="nav-item">
      <router-link
        to="/files"
        class="nav-link"
        :class="{
          active: currentRoute == 'Files',
          disabled: status != Status.STATUS_LOADED,
        }"
      >
        <i class="bi bi-files">&nbsp;</i>
        {{ $t("tab.files") }}
        <span v-if="filesIsModified">*</span>
      </router-link>
    </li>
    <li class="nav-item">
      <router-link
        to="/dex"
        class="nav-link"
        :class="{
          active: currentRoute == 'Dex',
          disabled: status != Status.STATUS_LOADED,
        }"
      >
        <i class="bi bi-gear">&nbsp;</i>
        {{ $t("tab.dex") }}
        <span v-if="dexIsModified">*</span>
      </router-link>
    </li>
    <li class="nav-item">
      <router-link
        to="/nsc"
        class="nav-link"
        :class="{
          active: currentRoute == 'NSC',
          disabled: status != Status.STATUS_LOADED,
        }"
      >
        <i class="bi bi-wifi">&nbsp;</i>
        {{ $t("tab.nsc") }}
        <span v-if="nscIsModified">*</span>
      </router-link>
    </li>
  </ul>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import Status from "@/typings/status";
import { useStore } from "@/store";
import { useRoute } from "vue-router";

export default defineComponent({
  name: "TabMenu",
  setup() {
    const store = useStore();
    const status = computed(() => store.getters["global/getStatus"]);
    const filesIsModified = computed(
      () => store.getters["files/isModified"]
    );
    const manifestIsModified = computed(
      () => store.getters["manifest/isModified"]
    );
    const dexIsModified = computed(() => store.getters["dex/isModified"]);
    const nscIsModified = computed(() => store.getters["nsc/isModified"]);

    const route = useRoute();
    const currentRoute = computed(() => route.name);

    return {
      status,
      filesIsModified,
      manifestIsModified,
      dexIsModified,
      nscIsModified,
      currentRoute,
    };
  },
  data() {
    return {
      Status,
    };
  },
});
</script>

<style>
.nav-tabs .nav-item .nav-link.active {
  background-color: #0d6efd;
  color: #ffffff;
}
</style>
