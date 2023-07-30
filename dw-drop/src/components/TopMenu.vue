<template>
  <nav class="navbar navbar-expand-md navbar-dark bg-dark p-2">
    <div class="container-fluid">
      <router-link
        to="/"
        class="navbar-brand"
      >
        DW-DROP
      </router-link>
      <ul class="navbar-nav">
        <li class="nav-item">
          <a
            class="nav-link"
            :class="{ disabled: locked }"
            href="#"
            @click="openApplicationDialog"
          >
            <i class="bi bi-folder2-open" />&nbsp;&nbsp;{{
              $t("topmenu.open")
            }}
          </a>
        </li>
        <li class="nav-item">
          <a
            class="nav-link"
            :class="{ disabled: locked || status != Status.STATUS_LOADED }"
            href="#"
            @click="saveApplicationDialog"
          >
            <i class="bi bi-save2" />&nbsp;&nbsp;{{ $t("topmenu.generate") }}
          </a>
        </li>
        <li class="nav-item">
          <a
            class="nav-link"
            :class="{ disabled: locked }"
            href="#"
            data-bs-toggle="modal"
            data-bs-target="#apkSignerModal"
          >
            <i class="bi bi-key" />&nbsp;&nbsp;{{ $t("topmenu.sign") }}
          </a>
        </li>
        <li class="nav-item">
          <a
            class="nav-link"
            href="#"
            data-bs-toggle="modal"
            data-bs-target="#contactForm"
          >
            <i class="bi bi-envelope" />&nbsp;&nbsp;{{
              $t("topmenu.contact")
            }}
          </a>
        </li>
        <li class="nav-item">
          <a
            class="nav-link"
            href="#"
            @click="openHelp"
          >
            <i class="bi bi-book" />&nbsp;&nbsp;{{ $t("topmenu.help") }}
          </a>
        </li>
      </ul>
    </div>
  </nav>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import { dialog, invoke } from "@tauri-apps/api";
import Status from "@/typings/status";
import Toast from "@/typings/toast";
import { useStore } from "@/store";
import { useRoute, useRouter } from "vue-router";

export default defineComponent({
  name: "TopMenu",
  setup() {
    const store = useStore();
    const status = computed(() => store.getters["global/getStatus"]);
    const locked = computed(() => store.getters["global/isLocked"]);
    const toast = (toast: Toast): void => {
      store.dispatch("global/toast", toast);
    };
    const openApplication = async (filename: string): Promise<void> => {
      return store.dispatch("global/openApplication", filename);
    };
    const saveApplication = (filename: string): void => {
      store.dispatch("global/commitAndSaveApplication", filename);
    };

    const route = useRoute();
    const router = useRouter();

    return {
      status,
      locked,
      toast,
      openApplication,
      saveApplication,
      route,
      router,
    };
  },
  data() {
    return {
      Status,
    };
  },
  methods: {
    async openApplicationDialog() {
      try {
        const filename = await dialog.open({
          filters: [
            {
              name: this.$t("apksign.app") + " (*.apk)",
              extensions: ["apk"],
            },
            {
              name: this.$t("files.all") + " (*)",
              extensions: ["*"],
            },
          ],
          multiple: false,
          directory: false,
        });
        if (filename) {
          await this.openApplication(filename as string);
          if (this.status != Status.STATUS_LOADED) {
            return;
          }
          if (this.route.name == "DragAndDrop") {
            this.router.push({ name: "Manifest" });
          }
        }
      } catch (error) {
        console.error(error);
        this.toast({ type: "error", message: error as string });
      }
    },
    async saveApplicationDialog() {
      try {
        const filename = await dialog.save({
          filters: [
            {
              name: this.$t("apksign.app") + " (*.apk)",
              extensions: ["apk"],
            },
            {
              name: this.$t("files.all") + " (*)",
              extensions: ["*"],
            },
          ],
        });
        if (filename) {
          this.saveApplication(filename as string);
        }
      } catch (error) {
        console.error(error);
        this.toast({ type: "error", message: error as string });
      }
    },
    async openHelp() {
      try {
        await invoke("open_help");
      } catch (error) {
        console.error(error);
        //context.dispatch("toast", { type: "error", message: error });
      }
    },
  },
});
</script>
