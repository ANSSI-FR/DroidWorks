<template>
  <div>
    <LoadingSpinner
      v-if="status == Status.STATUS_LOADING"
      :message="$t('loading.manifest')"
    />
    <div
      v-else
      id="accordionManifestPanel"
      class="accordion"
    >
      <div class="accordion-item">
        <h2
          id="applicationPanel-header"
          class="accordion-header"
        >
          <button
            class="accordion-button"
            type="button"
            data-bs-toggle="collapse"
            data-bs-target="#applicationPanel-collapse"
            aria-expanded="true"
            aria-controls="applicationPanel-collapse"
          >
            {{ $t("manifest.application") }}
          </button>
        </h2>
        <div
          id="applicationPanel-collapse"
          class="accordion-collapse collapse show"
          aria-labelledby="applicationPanel-header"
          data-bs-parent="#accordionManifestPanel"
        >
          <div class="accordion-body">
            <FlagList
              :infos="applicationInfos"
              :items="applicationFlags"
            />
          </div>
        </div>
      </div>
      <div
        v-for="collectionName in collectionNames"
        :key="collectionName"
        class="accordion-item"
      >
        <h2
          :id="collectionName + 'Panel-header'"
          class="accordion-header"
        >
          <button
            class="accordion-button collapsed"
            :class="{ empty: collections[collectionName].items.length == 0 }"
            :disabled="collections[collectionName].items.length == 0"
            type="button"
            data-bs-toggle="collapse"
            :data-bs-target="'#' + collectionName + 'Panel-collapse'"
            aria-expanded="false"
            :aria-controls="collectionName + 'Panel-collapse'"
          >
            <span style="text-transform: capitalize">
              {{
                $tc(
                  "manifest." + collectionName,
                  collections[collectionName].items.length
                )
              }}
            </span>
            &nbsp;&nbsp;
            <span
              class="badge"
              :class="isModified(collectionName) ? 'bg-danger' : 'bg-secondary'"
            >
              {{ collections[collectionName].items.length }}
            </span>
          </button>
        </h2>
        <div
          :id="collectionName + 'Panel-collapse'"
          class="accordion-collapse collapse"
          :aria-labelledby="collectionName + 'Panel-header'"
          data-bs-parent="#accordionManifestPanel"
        >
          <div class="accordion-body">
            <GenericList
              :key="collectionName"
              :collection-name="collectionName"
              :items="collections[collectionName].items"
              :newItems="newItems(collectionName)"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import LoadingSpinner from "@/components/LoadingSpinner.vue";
import GenericList from "@/components/manifest/GenericList.vue";
import FlagList from "@/components/manifest/FlagList.vue";
import Status from "@/typings/status";
import Item from "@/typings/manifest/item";
import { useStore } from "@/store";

export default defineComponent({
  name: "ManifestTab",
  components: {
    LoadingSpinner,
    GenericList,
    FlagList,
  },
  setup() {
    const store = useStore();
    const applicationInfos = computed(
      () => store.getters["manifest/getApplicationInfos"]
    );
    const applicationFlags = computed(
      () => store.getters["manifest/getApplicationFlags"]
    );
    const collectionNames = computed(
      () => store.getters["manifest/getCollectionNames"]
    );
    const collections = computed(() => store.getters["manifest/getData"]);
    const status = computed(() => store.getters["manifest/getAllDataStatus"]);
    const itemNamesToDrop = computed(
      () => store.getters["manifest/getItemNamesToDrop"]
    );
    const newPermissions = computed(
      () => store.getters["manifest/getNewPermissions"]
    );
    const fetchData = (): void => {
      store.dispatch("manifest/fetchData");
    };

    return {
      applicationInfos,
      applicationFlags,
      collectionNames,
      collections,
      status,
      itemNamesToDrop,
      newPermissions,
      fetchData,
    };
  },
  data() {
    return {
      Status,
    };
  },
  mounted() {
    this.fetchData();
  },
  methods: {
    isModified(collectionName: string) {
      if (this.itemNamesToDrop == null) {
        return false;
      }
      return (
        collectionName in this.itemNamesToDrop &&
        this.itemNamesToDrop[collectionName].size != 0
      );
    },
    newItems(collectionName: string): Item[] {
      if (collectionName == "permissions") {
        return this.newPermissions.map((permission: string) => {
          return {
            name: permission,
            isNew: true,
            attributes: {},
          };
        });
      } else {
        return [];
      }
    },
  },
});
</script>

<style>
button.empty.accordion-button.collapsed::after {
  display: none;
}
</style>
