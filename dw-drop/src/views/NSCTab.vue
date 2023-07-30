<template>
  <div>
    <LoadingSpinner
      v-if="listStatus == Status.STATUS_LOADING"
      :message="$t('loading.nsc')"
    />
    <div
      v-else
      class="list-group"
    >
      <div class="list-group-item">
        <select
          class="form-select"
          :disabled="locked"
          @change="changeSelection($event)"
        >
          <option
            v-for="(nsc, i) in availableNscs"
            :key="nsc"
            :value="i"
            :selected="nsc == currentNsc"
          >
            <span v-if="nsc.typ == 'original' && nsc.name">
              {{ nsc.name }} ({{ $t("default") }})
            </span>
            <span v-else-if="nsc.typ == 'custom'">
              {{ $t("nsc.custom." + nsc.name) }}
            </span>
            <span v-else>{{ $t("none") }}</span>
          </option>
        </select>
      </div>
      <LoadingSpinner
        v-if="currentStatus == Status.STATUS_LOADING"
        :message="$t('loading.nsc')"
      />
      <div
        v-else-if="xmlContent != undefined"
        class="list-group-item"
        style="white-space: pre-wrap"
      >
        {{ xmlContent }}
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import LoadingSpinner from "@/components/LoadingSpinner.vue";
import { NSCName } from "@/typings/nsc";
import Status from "@/typings/status";
import { useStore } from "@/store";

export default defineComponent({
  name: "NSCTab",
  components: {
    LoadingSpinner,
  },
  setup() {
    const store = useStore();
    const locked = computed(() => store.getters["global/isLocked"]);
    const availableNscs = computed(() => store.getters["nsc/getAvailableNscs"]);
    const listStatus = computed(() => store.getters["nsc/getListStatus"]);
    const currentNsc = computed(() => store.getters["nsc/getCurrentNsc"]);
    const xmlContent = computed(() => store.getters["nsc/getXmlContent"]);
    const currentStatus = computed(() => store.getters["nsc/getCurrentStatus"]);
    const fetchList = (): void => {
      store.dispatch("nsc/fetchList");
    };
    const selectNsc = (nsc: NSCName): void => {
      store.dispatch("nsc/selectNsc", nsc);
    };

    return {
      locked,
      availableNscs,
      listStatus,
      currentNsc,
      xmlContent,
      currentStatus,
      fetchList,
      selectNsc,
    };
  },
  data() {
    return {
      Status,
    };
  },
  mounted() {
    this.fetchList();
  },
  methods: {
    changeSelection(event: Event) {
      const i = (event.target as HTMLInputElement).value;
      this.selectNsc(this.availableNscs[i]);
    },
  },
});
</script>
