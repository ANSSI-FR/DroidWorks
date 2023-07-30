import { createStore, Store } from "vuex";
import globalModule, { GlobalState } from "./modules/global";
import filesModule, { FilesState } from "./modules/files";
import manifestModule, { ManifestState } from "./modules/manifest";
import dexModule, { DexState } from "./modules/dex";
import nscModule, { NSCState } from "./modules/nsc";

export interface State {
  global: GlobalState;
  files: FilesState;
  manifest: ManifestState;
  dex: DexState;
  nsc: NSCState;
}

export const store = createStore<State>({
  strict: true,

  modules: {
    global: globalModule,
    files: filesModule,
    manifest: manifestModule,
    dex: dexModule,
    nsc: nscModule,
  },
});

export function useStore() {
  return store as Store<State>;
}
