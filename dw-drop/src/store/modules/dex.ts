import { ActionContext } from "vuex";
import { tauri } from "@tauri-apps/api";
import Status from "@/typings/status";
import { State } from "@/store";

type Context = ActionContext<DexState, State>;

export interface DexState {
  nbStrings: number;
  strings: string[];
  stringsStatus: number;
}

interface DexStrings {
  nbTotal: number;
  data: string[];
}

export default {
  namespaced: true,
  state: (): DexState => ({
    nbStrings: 0,
    strings: [],
    stringsStatus: Status.STATUS_EMPTY,
  }),
  getters: {
    getNbStrings(state: DexState): number {
      return state.nbStrings;
    },
    getStrings(state: DexState): string[] {
      return state.strings;
    },
    getStringsStatus(state: DexState): Status {
      return state.stringsStatus;
    },
    isModified(state: DexState): boolean {
      state; /* getting rid of the unused parameter warning */
      return false;
    },
  },
  mutations: {
    reset(state: DexState) {
      state.strings = [];
      state.stringsStatus = Status.STATUS_EMPTY;
    },
    setNbStrings(state: DexState, nbStrings: number) {
      state.nbStrings = nbStrings;
    },
    setStrings(state: DexState, strings: string[]) {
      state.strings = strings;
    },
    setStringsStatus(state: DexState, status: Status) {
      state.stringsStatus = status;
    },
  },
  actions: {
    async fetchStrings(
      context: Context,
      req: { start: number; nb: number; filter: string }
    ) {
      context.commit("setStringsStatus", Status.STATUS_LOADING);
      try {
        const stringsResp = await tauri.invoke<DexStrings>("dex_strings", {
          req,
        });
        console.log("fetched " + stringsResp.data.length + " strings");
        context.commit("setNbStrings", stringsResp.nbTotal);
        context.commit("setStrings", stringsResp.data);
        context.commit("setStringsStatus", Status.STATUS_LOADED);
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
        context.commit("setStringsStatus", Status.STATUS_EMPTY);
      }
    },
  },
};
