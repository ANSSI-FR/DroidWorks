<template>
  <div>
    <div
      class="position-sticky vh-100 vw-100"
      :class="{ 'bg-light': status == Status.STATUS_EMPTY }"
    >
      <div class="container-fluid p-0">
        <div class="sticky-top">
          <TopMenu />
          <TabMenu />
        </div>
        <MsgToasts />

        <LoadingSpinner
          v-if="status == Status.STATUS_LOADING"
          :message="$t('loading')"
        />
        <router-view v-else />
      </div>
    </div>

    <ApkSignerModal />
    <ContactForm />
  </div>
</template>

<script lang="ts">
import "bootstrap/dist/css/bootstrap.min.css";
import "bootstrap/dist/js/bootstrap.min.js";
import "bootstrap-icons/font/bootstrap-icons.css";

import { computed, defineComponent } from "vue";
import TopMenu from "@/components/TopMenu.vue";
import TabMenu from "@/components/TabMenu.vue";
import ApkSignerModal from "@/components/ApkSignerModal.vue";
import ContactForm from "@/components/ContactForm.vue";
import LoadingSpinner from "@/components/LoadingSpinner.vue";
import MsgToasts from "@/components/MsgToasts.vue";
import Status from "@/typings/status";
import { useStore } from "@/store";
import { useRoute } from "vue-router";

export default defineComponent({
  name: "App",
  components: {
    TopMenu,
    TabMenu,
    ApkSignerModal,
    ContactForm,
    LoadingSpinner,
    MsgToasts,
  },
  setup() {
    const store = useStore();
    const status = computed(() => store.getters["global/getStatus"]);

    const route = useRoute();
    const currentRoute = computed(() => route.name);

    return {
      status,
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

<style scoped>
body {
  margin: 0;
  padding: 0;
}
</style>
