<template>
  <div>
    <div
      class="fixed-bottom d-flex flex-row bg-light justify-content-between align-items-center"
      style="height: 70px"
    >
      <span>&nbsp;</span>
      <span>{{ nbFiles }} {{ $tc("application.files", nbFiles) }}</span>
      <span>&nbsp;</span>
    </div>
    <LoadingSpinner
      v-if="status == Status.STATUS_LOADING"
      :message="$t('loading.files')"
    />
    <div
      v-else
      style="padding-bottom: 70px"
    >
      <div class="list-group">
        <FileItem
          v-for="file in files"
          :key="file.name"
          :file-entry="file"
          :depth="0"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent } from "vue";
import LoadingSpinner from "@/components/LoadingSpinner.vue";
import FileItem from "@/components/files/FileItem.vue";
import Status from "@/typings/status";
import { useStore } from "@/store";

export default defineComponent({
  name: "FilesTab",
  components: {
    LoadingSpinner,
    FileItem,
  },
  setup() {
    const store = useStore();
    const files = computed(() => store.getters["files/getFiles"]);
    const nbFiles = computed(() => store.getters["files/getNbFiles"]);
    const status = computed(() => store.getters["files/getFilesStatus"]);
    const fetchFiles = (): void => {
      store.dispatch("files/fetchFiles");
    };

    return { files, nbFiles, status, fetchFiles };
  },
  data() {
    return {
      Status,
    };
  },
  mounted() {
    this.fetchFiles();
  },
});
</script>
