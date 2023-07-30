<template>
  <div class="flex-fill">
    <ul class="list-group">
      <li
        v-if="collectionName == 'permissions'"
        class="list-group-item"
      >
        <div class="input-group">
          <input
            id="newPermission"
            v-model="newPermission"
            type="text"
            class="form-control"
            :placeholder="$t('manifest.permissions.new')"
            :disabled="locked"
          >
          <div class="tooltip-wrapper" :title="newPermError">
            <button
              class="btn btn-success"
              :class="{ disabled: locked || !validNewPermission }"
              :title="newPermError"
              @click="addPermission"
            >
              {{ $t("button.add") }}
            </button>
          </div>
        </div>
      </li>
      <ListItem
        v-for="item in newItems"
        :key="item.name"
        :item="item"
        :collection-name="collectionName"
      />
      <ListItem
        v-for="item in items"
        :key="item.name"
        :item="item"
        :collection-name="collectionName"
      />
    </ul>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent, PropType } from "vue";
import { useStore } from "@/store";
import ListItem from "@/components/manifest/ListItem.vue";
import Toast from "@/typings/toast";
import ManifestItem from "@/typings/manifest/item";

export default defineComponent({
  name: "GenericList",
  components: {
    ListItem,
  },
  props: {
    collectionName: {
      type: String,
      required: true,
    },
    items: Array as PropType<ManifestItem[]>,
    newItems: Array as PropType<ManifestItem[]>,
  },
  setup() {
    const store = useStore();
    const locked = computed(() => store.getters["global/isLocked"]);
    const doesPermissionExist = (permission: string) => {
      return store.getters["manifest/doesPermissionExist"](permission);
    };
    const addNewPermission = (permission: string): void => {
      store.commit("manifest/addNewPermission", permission);
    };
    const toast = (toast: Toast): void => {
      store.dispatch("global/toast", toast);
    };

    return { locked, doesPermissionExist, addNewPermission, toast };
  },
  data() {
    return {
      newPermission: "",
      validNewPermission: false,
      newPermError: this.$t("manifest.permissions.new.error.empty"),
    };
  },
  methods: {
    addPermission() {
      this.addNewPermission(this.newPermission);
      this.newPermission = "";
    },
  },
  watch: {
    newPermission(permission, _) {
      if (permission == "") {
        this.validNewPermission = false;
        this.newPermError = this.$t("manifest.permissions.new.error.empty");
        return;
      }
      let rePerm = /^[a-zA-Z0-9\$\-_]+(\.[a-zA-Z0-9\$\-_]+)*$/;
      if (!rePerm.test(permission)) {
        this.validNewPermission = false;
        this.newPermError = this.$t("manifest.permissions.new.error.malformed");
        return;
      }
      if (this.doesPermissionExist(permission)) {
        this.validNewPermission = false;
        this.newPermError = this.$t("manifest.permissions.new.error.existing");
        return;
      }
      this.validNewPermission = true;
      this.newPermError = "";
    },
  },
});
</script>
