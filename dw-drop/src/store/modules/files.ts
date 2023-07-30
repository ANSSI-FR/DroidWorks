import { ActionContext } from "vuex";
import { tauri } from "@tauri-apps/api";
import Status from "@/typings/status";
import { FileEntry, filenamesTree } from "@/typings/filenames";
import { State } from "@/store";

type Context = ActionContext<FilesState, State>;

export interface FilesState {
  files: FileEntry[];
  nbFiles: number;
  filesStatus: number;
  filesToDrop: Set<string>;
}

export default {
  namespaced: true,
  State: (): FilesState => ({
    files: [],
    nbFiles: 0,
    filesStatus: Status.STATUS_EMPTY,
    filesToDrop: new Set(),
  }),
  getters: {
    getFiles(state: FilesState): FileEntry[] {
      return state.files;
    },
    getNbFiles(state: FilesState): number {
      return state.nbFiles;
    },
    getFilesStatus(state: FilesState): Status {
      return state.filesStatus;
    },
    getFilesToDrop(state: FilesState): Set<string> {
      return state.filesToDrop;
    },
    isModified(state: FilesState): boolean {
      return state.filesToDrop && state.filesToDrop.size != 0;
    },
  },
  mutations: {
    reset(state: FilesState) {
      state.files = [];
      state.filesStatus = Status.STATUS_EMPTY;
      state.filesToDrop = new Set();
    },
    setFiles(state: FilesState, files: FileEntry[]) {
      state.files = files;
    },
    setNbFiles(state: FilesState, nbFiles: number) {
      state.nbFiles = nbFiles;
    },
    setFilesStatus(state: FilesState, status: Status) {
      state.filesStatus = status;
    },
    dropFile(state: FilesState, filename: string) {
      state.filesToDrop.add(filename);
    },
    keepFile(state: FilesState, filename: string) {
      state.filesToDrop.delete(filename);
    },
    resetFilesToDrop(state: FilesState) {
      state.filesToDrop.clear();
    },
  },
  actions: {
    async fetchFiles(context: Context) {
      if (context.state.filesStatus != Status.STATUS_EMPTY) {
        return;
      }
      context.commit("setFilesStatus", Status.STATUS_LOADING);
      try {
        const files = await tauri.invoke<{ name: string; size: number }[]>(
          "files"
        );
        console.log("fetched " + files.length + " file names");
        context.commit("setFiles", filenamesTree(files));
        context.commit("setNbFiles", files.length);
        context.commit("setFilesStatus", Status.STATUS_LOADED);
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
        context.commit("setFilesStatus", Status.STATUS_EMPTY);
      }
    },
  },
};
