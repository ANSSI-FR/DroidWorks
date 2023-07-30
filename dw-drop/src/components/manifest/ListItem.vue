<template>
  <li
    class="list-group-item"
    :class="{ 'list-group-item-dark': dropped }"
  >
    <div class="d-flex w-100 justify-content-between align-items-center">
      <div class="d-flex">
        <span
          style="overflow-wrap: anywhere"
          class="pe-3"
        >
          {{ name }}
        </span>
        <small class="fst-italic pt-1">
          {{ description }}
        </small>
      </div>
      <div>
        <span
          v-for="attribute in attributes()"
          :key="attribute.badgeName"
          class="badge border-dark mx-auto me-2 h-25"
          :class="attribute.badgeClass"
          :title="attribute.badgeTitle"
        >
          {{ $t("manifest.attribute." + attribute.badgeName) }}
        </span>
        <button
          v-if="dropped"
          class="btn btn-success"
          :class="{ disabled: locked || !hasName }"
          @click="keep"
        >
          {{ $t("button.keep") }}
        </button>
        <button
          v-else
          class="btn btn-danger"
          :class="{ disabled: locked || !hasName }"
          @click="drop"
        >
          {{ $t("button.remove") }}
        </button>
      </div>
    </div>
  </li>
</template>

<script lang="ts">
import { computed, defineComponent, PropType } from "vue";
import { PERMISSIONS } from "@/data/manifest/permissions";
import { FEATURES } from "@/data/manifest/features";
import { useStore } from "@/store";
import ManifestItem from "@/typings/manifest/item";
import { itemName } from "@/typings/manifest/item";
import { attributeIsTrue } from "@/typings/manifest/item";

export default defineComponent({
  name: "ListItem",
  props: {
    collectionName: {
      type: String,
      required: true,
    },
    item: {
      type: Object as PropType<ManifestItem>,
      required: true,
    },
  },
  setup() {
    const store = useStore();
    const locked = computed(() => store.getters["global/isLocked"]);
    const dropItemName = (collectionName: string, itemName: string): void => {
      store.commit("manifest/dropItemName", { collectionName, itemName });
    };
    const keepItemName = (collectionName: string, itemName: string): void => {
      store.commit("manifest/keepItemName", { collectionName, itemName });
    };
    const dropNewPermission = (name: string): void => {
      store.commit("manifest/dropNewPermission", name);
    };

    return { locked, dropItemName, keepItemName, dropNewPermission };
  },
  data() {
    return {
      dropped: false,
    };
  },
  computed: {
    hasName(): boolean {
      return !(itemName(this.item) == undefined);
    },
    name(): string {
      let n = itemName(this.item);
      if (n == undefined) {
        console.error("item must have a name: " + JSON.stringify(this.item));
        return "<unnamed>";
      }
      return n;
    },
    description(): string | null {
      let obj = null;
      switch (this.collectionName) {
        case "permissions": {
          obj = PERMISSIONS[this.name];
          break;
        }
        case "features": {
          obj = FEATURES[this.name];
          break;
        }
      }
      if (obj && obj.description) {
        return obj.description;
      } else {
        return null;
      }
    },
  },
  methods: {
    drop() {
      if (this.collectionName == "permissions" && this.item.isNew == true) {
        this.dropNewPermission(this.name);
      } else {
        this.dropItemName(this.collectionName, this.name);
        this.dropped = true;
      }
    },
    keep() {
      this.keepItemName(this.collectionName, this.name);
      this.dropped = false;
    },
    attributes(): { badgeName: string, badgeClass: string, badgeTitle: string }[] {
      const attrNames: string[] = ["enabled", "exported", "required"];
      let attrs = [];
      for (const attrName of attrNames) {
        const value = attributeIsTrue(this.item, attrName);
        if (value == undefined) {
          attrs.push({
            badgeName: attrName,
            badgeClass: "bg-warning text-dark attr-dynamic",
            badgeTitle: this.$t("manifest.attribute.dynamic"),
          });
        } else if (value) {
          attrs.push({
            badgeName: attrName,
            badgeClass: "bg-warning text-dark attr-static",
            badgeTitle: this.$t("manifest.attribute.static"),
          });
        }
      }
      if (this.item.isNew == true) {
        attrs.push({
          badgeName: "new",
          badgeClass: "bg-primary text-white",
          badgeTitle: "",
        });
      }
      return attrs;
    },
  },
});
</script>

<style>
.attr-static {
  border: 0px;
}

.attr-dynamic {
  border-style: dashed;
  border-width: 2px;
  border-color: black;
}
</style>
