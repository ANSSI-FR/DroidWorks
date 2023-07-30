<template>
  <div>
    <div
      class="list-group-item"
      :class="{ 'list-group-item-dark': dropped }"
    >
      <div class="d-flex w-100 justify-content-between align-items-center">
        <span
          :style="indent"
          style="overflow-wrap: anywhere"
          class="pe-1"
        >
          <button
            v-if="isDir"
            class="btn ps-0 pe-1"
            data-bs-toggle="collapse"
            :data-bs-target="collapseTarget"
            role="button"
            @click="showChildren = !showChildren"
          >
            <i
              v-if="showChildren"
              class="bi bi-caret-down-fill"
            />
            <i
              v-else
              class="bi bi-caret-right-fill"
            />
          </button>
          <span
            v-if="!isFile"
            class="ms-1 badge bg-light text-dark"
          >
            {{ fileEntry.children.length }}
          </span>
          <i
            v-else
            class="bi pe-1"
            :class="extIcon"
          />
          {{ fileEntry.name }}
          <span
            v-if="isFile"
            class="ms-3 badge bg-light text-dark"
          >
            {{ fileEntry.size }}
          </span>
        </span>

        <span v-if="isFile">
          <button
            v-if="isFileImage"
            :id="'btn_' + uid"
            class="btn btn-light"
            :class="{ disabled: locked }"
            @click="createPreview"
            @mouseleave="destroyPreview"
          >
            <i class="bi bi-file-earmark-image" />
          </button>
          <div
            v-if="isFileImage"
            :id="'ttp_' + uid"
            role="tooltip"
            class="popper-tooltip"
          >
            <img :id="'img_' + uid">
            <div
              class="popper-arrow"
              data-popper-arrow
            />
          </div>
          &nbsp;
          <button
            v-if="dropped"
            class="btn btn-success"
            :class="{ disabled: locked }"
            :title="$t('button.restore')"
            @click="keep"
          >
            <i class="bi bi-arrow-90deg-left" />
          </button>
          <button
            v-else
            class="btn btn-danger"
            :class="{ disabled: locked || isIrremovable }"
            :title="$t('button.remove')"
            @click="drop"
          >
            <i class="bi bi-trash" />
          </button>
          &nbsp;
          <button
            class="btn btn-secondary"
            :class="{ disabled: locked }"
            :title="$t('button.replace') + '...'"
            @click="replace"
          >
            <i class="bi bi-arrow-left-right" />
          </button>
          &nbsp;
          <button
            class="btn btn-secondary"
            :class="{ disabled: locked }"
            :title="$t('button.extract') + '...'"
            @click="extract"
          >
            <i class="bi bi-box-arrow-up" />
          </button>
        </span>
      </div>
    </div>
    <div
      v-if="!isFile"
      :id="collapseSelector"
      class="list-group collapse"
    >
      <FileItem
        v-for="child in fileEntry.children"
        :key="fullPath + '/' + child.name"
        :file-entry="child"
        :path="fullPath"
        :depth="depth + 1"
      />
    </div>
  </div>
</template>

<script lang="ts">
import { computed, defineComponent, PropType } from "vue";
import { uid } from "uid";
import { dialog, fs, tauri } from "@tauri-apps/api";
import { createPopper, Instance as PopperInstance } from "@popperjs/core";
import Toast from "@/typings/toast";
import { FileExtension, FileEntry, fileExtension } from "@/typings/filenames";
import { useStore } from "@/store";

export default defineComponent({
  name: "FileItem",
  props: {
    path: String,
    fileEntry: {
      type: Object as PropType<FileEntry>,
      required: true,
    },
    depth: {
      type: Number,
      required: true,
    },
  },
  setup() {
    const store = useStore();
    const locked = computed(() => store.getters["global/isLocked"]);
    const toast = (toast: Toast): void => {
      store.dispatch("global/toast", toast);
    };
    const dropFile = (filename: string): void => {
      store.commit("files/dropFile", filename);
    };
    const keepFile = (filename: string): void => {
      store.commit("files/keepFile", filename);
    };

    return { locked, toast, dropFile, keepFile };
  },
  data() {
    return {
      uid: uid(16),
      dropped: false,
      showChildren: false,
      popperInstance: null as PopperInstance | null,
      popperTooltip: null as HTMLElement | null,
    };
  },
  computed: {
    indent() {
      if (this.depth == undefined) {
        console.error("undefined file tree depth");
        return {};
      }
      return { "padding-left": `${this.depth * 32}px` };
    },
    fullPath(): string {
      if (this.fileEntry == undefined) {
        console.error("undefined filename");
        return "";
      }
      const path = this.path;
      if (this.path == undefined) {
        return this.fileEntry.name;
      }
      return path + "/" + this.fileEntry.name;
    },
    isFile(): boolean {
      if (this.fileEntry == undefined) {
        console.error("undefined filename");
        return true;
      }
      return this.fileEntry.children.length == 0;
    },
    isDir(): boolean {
      return !this.isFile;
    },
    isFileImage(): boolean {
      if (this.fileEntry == undefined) {
        return false;
      }
      return fileExtension(this.fileEntry.name) == FileExtension.Image;
    },
    extIcon() {
      if (this.fileEntry == undefined) {
        return { "bi-file-earmark-fill": true };
      }
      const ext = fileExtension(this.fileEntry.name);
      switch (ext) {
        case FileExtension.Image:
          return ["bi-file-earmark-image-fill"];
        case FileExtension.Xml:
          return ["bi-file-earmark-code-fill"];
        case FileExtension.Dex:
          return ["bi-file-earmark-binary-fill"];
        case FileExtension.Other:
        default:
          return ["bi-file-earmark-fill"];
      }
    },
    isIrremovable() {
      switch (this.fullPath) {
        case "AndroidManifest.xml":
        case "resources.arsc":
          return true;
        default:
          return this.fullPath.match(/^classes[0-9]*\.dex$/);
      }
    },
    collapseSelector() {
      return "s_" + this.uid;
    },
    collapseTarget() {
      return "#" + this.collapseSelector;
    },
  },
  methods: {
    createPreview() {
      if (!this.isFileImage) {
        return;
      }
      const button = document.querySelector("#btn_" + this.uid);
      if (button == undefined) {
        return;
      }
      const tooltip = document.querySelector("#ttp_" + this.uid);
      if (tooltip == undefined) {
        return;
      }
      const image = document.querySelector("#img_" + this.uid);
      if (image == undefined) {
        return;
      }
      const img = image as HTMLImageElement;
      tauri
        .invoke<string>("file", { asset: this.fullPath })
        .then((result) => {
          img.src = "data:image/png;base64," + result;
          this.popperInstance = createPopper(button, tooltip as HTMLElement, {
            placement: "left",
          });
          tooltip.setAttribute("data-show", "");
          this.popperTooltip = tooltip as HTMLElement;
        })
        .catch((error) => {
          console.error(error);
        });
    },
    destroyPreview() {
      if (this.popperInstance == null || this.popperTooltip == null) {
        return;
      }
      this.popperTooltip.removeAttribute("data-show");
      this.popperInstance.destroy();
    },
    drop() {
      if (this.fullPath) {
        this.dropFile(this.fullPath);
        this.dropped = true;
      }
    },
    keep() {
      if (this.fullPath) {
        this.keepFile(this.fullPath);
        this.dropped = false;
      }
    },
    replace() {
      let name = this.fullPath;
      const sname1 = name.split("\\").pop();
      if (sname1 == undefined) {
        console.error("filename undefined after split on '\\'");
        return;
      }
      const sname2 = sname1.split("/").pop();
      if (sname2 == undefined) {
        console.error("filename undefined after split on '/'");
        return;
      }
      name = sname2;
      console.log("filename: " + name);
      let extension = "*";
      if (name.includes(".")) {
        const ext = name.split(".").pop();
        console.log("ext: " + ext);
        if (ext) {
          extension = ext;
        }
      }
      console.log("extension: " + extension);
      const extensions = [];
      extensions.push(extension);
      dialog
        .open({
          filters: [{ name, extensions }],
          multiple: false,
          directory: false,
        })
        .then((filename) => {
          if (filename) {
            // filename should be string and not string[] due to 'multiple: false' flag.
            fs.readBinaryFile(filename as string)
              .then((result) => {
                console.log(result);
                let binary = "";
                for (let i = 0; i < result.length; i++) {
                  binary += String.fromCharCode(result[i]);
                }
                const b64 = window.btoa(binary);
                tauri
                  .invoke<null>("replace_file_other", {
                    asset: this.fullPath,
                    content: b64,
                  })
                  .then((result) => {
                    console.log(result);
                  })
                  .catch((error) => {
                    console.error(error);
                    this.toast({
                      type: "error",
                      message: error,
                    });
                  });
              })
              .catch((error) => {
                console.error(error);
                this.toast({ type: "error", message: error });
              });
          }
        })
        .catch((error) => {
          console.error(error);
          this.toast({ type: "error", message: error });
        });
    },
    extract() {
      let name = this.fullPath;
      const sname1 = name.split("\\").pop();
      if (sname1 == undefined) {
        console.error("filename undefined after split on '\\'");
        return;
      }
      const sname2 = sname1.split("/").pop();
      if (sname2 == undefined) {
        console.error("filename undefined after split on '/'");
        return;
      }
      name = sname2;
      console.log("filename: " + name);
      dialog
        .save({
          defaultPath: name,
          filters: [],
        })
        .then((filename) => {
          if (filename != null) {
            tauri
              .invoke<null>("extract_file", { asset: this.fullPath, filename })
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
  },
});
</script>

<style scoped>
img {
  background-color: #cccccc;
}

.btn:focus,
.btn:active {
  outline: none !important;
  box-shadow: none;
}

.popper-tooltip {
  background: #333;
  color: white;
  font-weight: bold;
  padding: 4px 8px;
  font-size: 13px;
  border-radius: 4px;
  z-index: 12000;
  display: none;
}

.popper-tooltip[data-show] {
  display: block;
}

.popper-arrow,
.popper-arrow::before {
  position: absolute;
  width: 8px;
  height: 8px;
  background: inherit;
}

.popper-arrow {
  visibility: hidden;
}

.popper-arrow::before {
  visibility: visible;
  content: "";
  transform: rotate(45deg);
}

.popper-tooltip[data-popper-placement^="top"] > .popper-arrow {
  bottom: -4px;
}

.popper-tooltip[data-popper-placement^="bottom"] > .popper-arrow {
  top: -4px;
}

.popper-tooltip[data-popper-placement^="left"] > .popper-arrow {
  right: -4px;
}

.popper-tooltip[data-popper-placement^="right"] > .popper-arrow {
  left: -4px;
}
</style>
