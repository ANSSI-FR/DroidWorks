<template>
  <div>
    <div
      class="fixed-bottom d-flex flex-row bg-light justify-content-between align-items-center"
      style="height: 70px"
    >
      <div class="input-group w-25 ps-3">
        <input
          v-model="filter"
          type="text"
          class="form-control disabled"
          placeholder=""
          @keyup.enter="fetch"
        >
        <button
          class="btn btn-outline-secondary"
          type="button"
          @click="fetch"
        >
          <i class="bi bi-search" />
        </button>
      </div>

      <div>
        <a
          v-if="canPrevious()"
          href="#"
          class="btn p-0 pb-1"
          @click="pageBegin"
        >
          <i class="bi bi-chevron-double-left" />
        </a>
        <a
          v-if="canPrevious()"
          href="#"
          class="btn p-0 pb-1"
          @click="pagePrevious"
        >
          <i class="bi bi-chevron-left" />
        </a>
        {{ start }} - {{ start + strings.length }} / {{ nbStrings }}
        <a
          v-if="canNext()"
          href="#"
          class="btn p-0 pb-1"
          @click="pageNext"
        >
          <i class="bi bi-chevron-right" /></a>
        <a
          v-if="canNext()"
          href="#"
          class="btn p-0 pb-1"
          @click="pageEnd"
        >
          <i class="bi bi-chevron-double-right" />
        </a>
      </div>
      <button
        class="p-1 m-3"
        @click="extract"
      >
        {{ $t("button.dex.strings") }}
      </button>
    </div>
    <LoadingSpinner
      v-if="status == Status.STATUS_LOADING"
      :message="$t('loading.dex')"
    />
    <div
      v-else
      style="padding-bottom: 70px"
    >
      <div class="list-group">
        <div
          v-for="(string, i) in strings"
          :key="string"
          class="list-group-item"
        >
          <div class="d-flex w-100 justify-content-between align-items-center">
            <div>
              <span class="badge bg-secondary me-1">
                {{ start + i }}
              </span>
              <span
                v-if="!filtered"
                style="overflow-wrap: anywhere"
                class="pe-1"
              >
                {{ string }}
              </span>
              <span
                v-else
                style="overflow-wrap: anywhere"
                class="pe-1"
              >
                {{ string.split(filtered, 2)[0] }}<b>{{ filtered }}</b>{{ string.split(filtered, 2)[1] }}
              </span>
            </div>
            <button
              class="btn btn-light"
              @click="clipboardCopy(string)"
            >
              <i class="bi bi-clipboard" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import { clipboard, dialog, tauri } from "@tauri-apps/api";
import LoadingSpinner from "@/components/LoadingSpinner.vue";
import Status from "@/typings/status";
import Toast from "@/typings/toast";
import { useStore } from "@/store";

export default defineComponent({
  name: "DexTab",
  components: {
    LoadingSpinner,
  },
  setup() {
    const store = useStore();
    const nbStrings = computed(() => store.getters["dex/getNbStrings"]);
    const strings = computed(() => store.getters["dex/getStrings"]);
    const status = computed(() => store.getters["dex/getStringsStatus"]);
    const fetchStrings = (start: number, nb: number, filter: string): void => {
      store.dispatch("dex/fetchStrings", { start, nb, filter });
    };
    const toast = (toast: Toast): void => {
      store.dispatch("global/toast", toast);
    };

    return { nbStrings, strings, status, fetchStrings, toast };
  },
  data() {
    return {
      Status,
      start: 0,
      nb: 100,
      filter: "",
      filtered: "",
    };
  },
  mounted() {
    this.fetch();
  },
  methods: {
    fetch() {
      let nb = this.nb;
      if (this.nbStrings > 0) {
        nb = Math.min(nb, this.nbStrings - this.start);
      }
      this.fetchStrings(this.start, nb, this.filter);
      this.filtered = this.filter;
    },
    extract() {
      dialog.save({ filters: [] }).then((filename) => {
        if (filename != null) {
          tauri
            .invoke<null>("extract_dex_strings", { filename })
            .then((result) => {
              console.log(result);
              this.toast({
                type: "success",
                message: "Fichier sauvegardé dans '" + filename + "'.",
              });
            })
            .catch((error) => {
              console.error(error);
              this.toast({ type: "error", message: error });
            });
        }
      });
    },
    clipboardCopy(s: string) {
      clipboard
        .writeText(s)
        .then((result) => {
          console.log(result);
        })
        .catch((error) => {
          console.error(error);
        });
    },
    canPrevious(): boolean {
      return this.start > 0;
    },
    canNext(): boolean {
      return this.start + this.nb < this.nbStrings;
    },
    pagePrevious() {
      this.start -= this.nb;
      if (this.start < 0) {
        this.start = 0;
      }
      this.fetch();
    },
    pageNext() {
      this.start += this.nb;
      this.fetch();
    },
    pageBegin() {
      this.start = 0;
      this.fetch();
    },
    pageEnd() {
      this.start +=
        Math.floor((this.nbStrings - this.start) / this.nb) * this.nb;
      this.fetch();
    },
  },
});
</script>
