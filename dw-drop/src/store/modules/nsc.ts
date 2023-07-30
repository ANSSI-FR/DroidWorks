import { ActionContext } from "vuex";
import vkbeautify from "vkbeautify";
import { tauri } from "@tauri-apps/api";
import Status from "@/typings/status";
import { NSCName } from "@/typings/nsc";
import { State } from "@/store";

type Context = ActionContext<NSCState, State>;

export interface NSCState {
  availableNscs: NSCName[];
  listStatus: Status;
  currentNsc?: NSCName;
  xmlContent?: string;
  currentStatus: Status;
}

export default {
  namespaced: true,
  state: (): NSCState => ({
    availableNscs: [],
    listStatus: Status.STATUS_EMPTY,
    currentNsc: undefined,
    xmlContent: "",
    currentStatus: Status.STATUS_EMPTY,
  }),
  getters: {
    getAvailableNscs(state: NSCState): NSCName[] {
      return state.availableNscs;
    },
    getListStatus(state: NSCState): Status {
      return state.listStatus;
    },
    getCurrentNsc(state: NSCState): NSCName | undefined {
      return state.currentNsc;
    },
    getXmlContent(state: NSCState): string | undefined {
      return state.xmlContent;
    },
    getCurrentStatus(state: NSCState): Status {
      return state.currentStatus;
    },
    isModified(state: NSCState): boolean {
      return (
        state.currentNsc != undefined && state.currentNsc.typ != "original"
      );
    },
  },
  mutations: {
    reset(state: NSCState) {
      state.availableNscs = [];
      state.listStatus = Status.STATUS_EMPTY;
      state.currentNsc = undefined;
      state.xmlContent = "";
      state.currentStatus = Status.STATUS_EMPTY;
    },
    setAvailableNscs(state: NSCState, nscs: NSCName[]) {
      state.availableNscs = nscs;
    },
    setListStatus(state: NSCState, status: Status) {
      state.listStatus = status;
    },
    setCurrentNsc(state: NSCState, nsc: NSCName) {
      state.currentNsc = nsc;
    },
    setXmlContent(state: NSCState, content: string | undefined) {
      state.xmlContent = content;
    },
    setCurrentStatus(state: NSCState, status: Status) {
      state.currentStatus = status;
    },
  },
  actions: {
    async fetchList(context: Context) {
      if (context.getters.getListStatus != Status.STATUS_EMPTY) {
        return;
      }
      context.commit("setListStatus", Status.STATUS_LOADING);
      try {
        const nscs = await tauri.invoke<NSCName[]>("available_nscs");
        console.log("available_nscs: " + JSON.stringify(nscs));
        context.commit("setAvailableNscs", nscs);
        context.commit("setListStatus", Status.STATUS_LOADED);
        if (nscs.length > 0) {
          context.dispatch("selectNsc", nscs[0]);
        }
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
        context.commit("setListStatus", Status.STATUS_EMPTY);
      }
    },
    async selectNsc(context: Context, nsc: NSCName) {
      if (nsc == context.getters.getCurrentNsc) {
        return;
      }
      context.commit("setCurrentStatus", Status.STATUS_LOADING);
      try {
        const content = await tauri.invoke<string | undefined>("get_nsc", {
          nsc,
        });
        console.log("nsc: " + content);
        const beautified =
          content == undefined ? undefined : vkbeautify.xml(content, 4);
        context.commit("setCurrentNsc", nsc);
        context.commit("setXmlContent", beautified);
        context.commit("setCurrentStatus", Status.STATUS_LOADED);
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
        context.commit("setCurrentStatus", Status.STATUS_EMPTY);
      }
    },
  },
};
