import { ActionContext } from "vuex";
import { tauri } from "@tauri-apps/api";
import ManifestItem from "@/typings/manifest/item";
import { itemName } from "@/typings/manifest/item";
import Info from "@/typings/manifest/info";
import Flag from "@/typings/manifest/flag";
import Status from "@/typings/status";
import { State } from "@/store";

type Context = ActionContext<ManifestState, State>;

const collectionNames = [
  "activities",
  "features",
  "permissions",
  "providers",
  "receivers",
  "services",
];

export interface ManifestState {
  applicationInfos: Info;
  applicationFlags: Flag[];
  applicationStatus: Status;
  collectionNames: string[];
  data: { [key: string]: { status: Status; items: ManifestItem[] } };
  itemNamesToDrop: { [key: string]: Set<string> };
  newPermissions: string[];
}

export default {
  namespaced: true,
  state: (): ManifestState => ({
    applicationInfos: {
      package: undefined,
      versionCode: undefined,
      versionName: undefined,
    },
    applicationFlags: [],
    applicationStatus: Status.STATUS_EMPTY,
    collectionNames,
    data: Object.fromEntries(
      collectionNames.map((collectionName) => [
        collectionName,
        {
          status: Status.STATUS_EMPTY,
          items: [],
        },
      ])
    ),
    itemNamesToDrop: Object.fromEntries(
      collectionNames.map((collectionName) => [collectionName, new Set()])
    ),
    newPermissions: [],
  }),
  getters: {
    getApplicationInfos(state: ManifestState): Info {
      return state.applicationInfos;
    },
    getApplicationFlags(state: ManifestState): Flag[] {
      return state.applicationFlags;
    },
    getCollectionNames(state: ManifestState): string[] {
      return state.collectionNames;
    },
    getData(state: ManifestState): {
      [key: string]: { status: Status; items: ManifestItem[] };
    } {
      return state.data;
    },
    getAllDataStatus(state: ManifestState): Status {
      const statuses = collectionNames.map((collectionName) => {
        return state.data[collectionName].status;
      });
      statuses.push(state.applicationStatus);
      return Math.min(...statuses);
    },
    getItemNamesToDrop(state: ManifestState): {
      [key: string]: Set<string>;
    } {
      return state.itemNamesToDrop;
    },
    getNewPermissions(state: ManifestState): string[] {
      return state.newPermissions;
    },
    doesPermissionExist: (state: ManifestState) => (permission: string): boolean => {
      if (state.newPermissions.includes(permission)) {
        return true;
      }
      if ("permissions" in state.data) {
        return state.data["permissions"].items.find((item: ManifestItem) => {
          return itemName(item) == permission;
        }) != undefined;
      } else {
        return false;
      }
    },
    isModified(state: ManifestState): boolean {
      if (state.newPermissions.length > 0) {
        return true;
      }
      for (const collectionName in state.itemNamesToDrop) {
        if (state.itemNamesToDrop[collectionName].size != 0) {
          return true;
        }
      }
      for (const flag of state.applicationFlags) {
        if (flag.value !== flag.previous) {
          return true;
        }
      }
      return false;
    },
  },
  mutations: {
    reset(state: ManifestState) {
      state.applicationInfos = {
        package: undefined,
        versionCode: undefined,
        versionName: undefined,
      };
      state.applicationFlags = [];
      for (const collectionName of state.collectionNames) {
        state.data[collectionName] = {
          status: Status.STATUS_EMPTY,
          items: [],
        };
        state.newPermissions = [];
        state.itemNamesToDrop[collectionName] = new Set();
      }
    },
    setApplicationInfos(state: ManifestState, infos: Info) {
      state.applicationInfos = infos;
    },
    setApplicationFlags(state: ManifestState, flags: Flag[]) {
      state.applicationFlags = flags;
    },
    setApplicationFlagValue(
      state: ManifestState,
      payload: { name: string; value: boolean | undefined }
    ) {
      for (const i in state.applicationFlags) {
        if (state.applicationFlags[i].name == payload.name) {
          state.applicationFlags[i].value = payload.value;
        }
      }
    },
    setApplicationStatus(state: ManifestState, status: Status) {
      state.applicationStatus = status;
    },
    setDataItems(
      state: ManifestState,
      payload: { collectionName: string; items: ManifestItem[] }
    ) {
      state.data[payload.collectionName].items = payload.items;
    },
    setDataStatus(
      state: ManifestState,
      payload: { collectionName: string; status: Status }
    ) {
      state.data[payload.collectionName].status = payload.status;
    },
    dropItemName(
      state: ManifestState,
      payload: { collectionName: string; itemName: string }
    ) {
      state.itemNamesToDrop[payload.collectionName].add(payload.itemName);
    },
    keepItemName(
      state: ManifestState,
      payload: { collectionName: string; itemName: string }
    ) {
      state.itemNamesToDrop[payload.collectionName].delete(payload.itemName);
    },
    addNewPermission(state: ManifestState, permission: string) {
      state.newPermissions.unshift(permission);
    },
    dropNewPermission(state: ManifestState, permission: string) {
      for (let i = 0; i < state.newPermissions.length; i++) {
        if (state.newPermissions[i] == permission) {
          state.newPermissions.splice(i, 1);
          break;
        }
      }
    },
    resetItemNamesToDrop(state: ManifestState) {
      for (const collectionName of state.collectionNames) {
        state.itemNamesToDrop[collectionName].clear();
      }
    },
  },
  actions: {
    async fetchData(context: Context) {
      if (context.getters.getAllDataStatus != Status.STATUS_EMPTY) {
        return;
      }
      try {
        context.commit("setApplicationStatus", Status.STATUS_LOADING);
        const resInfos = await tauri.invoke<Info>("application_infos");
        console.log("applicationInfos: " + JSON.stringify(resInfos));
        context.commit("setApplicationInfos", resInfos);

        const resFlags = await tauri.invoke<Flag[]>("application_flags");
        console.log("applicationFlags: " + JSON.stringify(resFlags));
        context.commit("setApplicationFlags", resFlags);
        context.commit("setApplicationStatus", Status.STATUS_LOADED);
      } catch (error) {
        console.error(error);
        context.dispatch(
          "global/toast",
          {
            type: "error",
            message: error,
          },
          { root: true }
        );
        context.commit("setApplicationStatus", Status.STATUS_EMPTY);
      }
      for (const collectionName of collectionNames) {
        context.commit("setDataStatus", {
          collectionName,
          status: Status.STATUS_LOADING,
        });
        try {
          const result = await tauri.invoke<ManifestItem[]>(collectionName);
          console.log(collectionName + ": " + JSON.stringify(result));
          context.commit("setDataItems", {
            collectionName,
            items: result,
          });
          context.commit("setDataStatus", {
            collectionName,
            status: Status.STATUS_LOADED,
          });
        } catch (error) {
          console.error(error);
          context.dispatch(
            "global/toast",
            {
              type: "error",
              message: error,
            },
            { root: true }
          );
          context.commit("setDataStatus", {
            collectionName,
            status: Status.STATUS_EMPTY,
          });
        }
      }
    },
  },
};
