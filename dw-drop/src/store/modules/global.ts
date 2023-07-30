import { ActionContext } from "vuex";
import { tauri } from "@tauri-apps/api";
import mitt from "mitt";
import { Handler } from "mitt";
import i18n from "@/i18n";
import Toast from "@/typings/toast";
import Status from "@/typings/status";
import { State } from "@/store";

type Context = ActionContext<GlobalState, State>;

type Events = {
  toast: Toast;
};
const emitter = mitt<Events>();

export interface GlobalState {
  status: number;
  toastEmitter: typeof emitter;
  locked: boolean;
}

export default {
  namespaced: true,
  state: (): GlobalState => ({
    status: Status.STATUS_EMPTY,
    toastEmitter: emitter,
    locked: false,
  }),
  getters: {
    getStatus(state: GlobalState): Status {
      return state.status;
    },
    isLocked(state: GlobalState): boolean {
      return state.locked;
    },
  },
  mutations: {
    setStatus(state: GlobalState, status: Status) {
      state.status = status;
    },
    lock(state: GlobalState) {
      state.locked = true;
    },
    unlock(state: GlobalState) {
      state.locked = false;
    },
  },
  actions: {
    toast(context: Context, toast: Toast) {
      context.state.toastEmitter.emit("toast", toast);
    },
    toasterSubscribe(context: Context, handler: Handler<Toast>) {
      context.state.toastEmitter.on("toast", handler);
    },
    async openApplication(context: Context, filename: string) {
      const previousState = context.state.status;
      context.commit("setStatus", Status.STATUS_LOADING);
      try {
        const result = await tauri.invoke<null>("open_application", {
          filename,
        });
        console.log(result);
        context.dispatch("toast", {
          type: "success",
          message: i18n.global.t("loaded", { filename }),
        });
        context.commit("files/reset", null, { root: true });
        context.commit("manifest/reset", null, { root: true });
        context.commit("dex/reset", null, { root: true });
        context.commit("nsc/reset", null, { root: true });
        context.commit("setStatus", Status.STATUS_LOADED);
      } catch (error) {
        console.error(error);
        context.dispatch("toast", { type: "error", message: error });
        context.commit("setStatus", previousState);
      }
    },
    async commitAndSaveApplication(context: Context, filename: string) {
      try {
        context.commit("lock");

        /* remove files from apk */
        if (context.rootGetters["files/isModified"]) {
          const filesToDrop = context.rootGetters["files/getFilesToDrop"];
          console.log("filesToDrop: " + Array.from(filesToDrop));
          await tauri.invoke<null>("remove_files", {
            assets: Array.from(filesToDrop),
          });
          console.log("files effectively dropped");
          context.commit("files/reset", null, { root: true });
          context.dispatch("files/fetchFiles", null, { root: true });
        }

        if (context.rootGetters["manifest/isModified"]) {
          /* set application flags */
          await tauri.invoke<null>("set_application_flags", {
            flags: context.rootGetters["manifest/getApplicationFlags"],
          });

          /* drop items from manifest */
          const itemNamesToDrop =
            context.rootGetters["manifest/getItemNamesToDrop"];
          console.log(itemNamesToDrop);
          for (const collectionName in itemNamesToDrop) {
            if (itemNamesToDrop[collectionName].size == 0) {
              console.log(collectionName + ", nothing to drop");
            } else {
              console.log(
                collectionName +
                  ", dropping: " +
                  Array.from(itemNamesToDrop[collectionName])
              );
              await tauri.invoke<null>("drop_" + collectionName, {
                items: Array.from(itemNamesToDrop[collectionName]),
              });
            }
          }
          
          // add new permissions
          const newPermissions =
            context.rootGetters["manifest/getNewPermissions"];
          console.log("adding permissions: " + JSON.stringify(newPermissions));
          await tauri.invoke<null>("add_permissions", {
            items: newPermissions,
          });

          context.commit("manifest/reset", null, { root: true });
          context.dispatch("manifest/fetchData", null, { root: true });
        }

        /* drop network security config attribute from manifest */
        if (context.rootGetters["nsc/isModified"]) {
          const nsc = context.rootGetters["nsc/getCurrentNsc"];
          console.log("sending nsc to backend: " + nsc);
          await tauri.invoke<null>("commit_nsc", { nsc });
          context.commit("nsc/reset", null, { root: true });
          context.dispatch("nsc/fetchList", null, { root: true });
        }

        /* asking the backend to write file */
        console.log("writing file " + filename + "...");
        context.dispatch("toast", {
          type: "information",
          message: i18n.global.t("generate", { filename }),
          sticky: true,
        });
        await tauri.invoke<null>("save_application", { filename });
        console.log("ok");
        context.commit("unlock");
        context.dispatch("toast", {
          type: "success",
          message: i18n.global.t("generated", { filename }),
        });
      } catch (error) {
        console.error(error);
        context.dispatch("toast", { type: "error", message: error });
        context.commit("unlock");
      }
    },
  },
};
