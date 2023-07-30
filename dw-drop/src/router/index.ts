import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router";
import DragAndDrop from "@/views/DragAndDrop.vue";
import FilesTab from "@/views/FilesTab.vue";
import ManifestTab from "@/views/ManifestTab.vue";
import DexTab from "@/views/DexTab.vue";
import NSCTab from "@/views/NSCTab.vue";

const routes: Array<RouteRecordRaw> = [
  {
    path: "/",
    name: "DragAndDrop",
    component: DragAndDrop,
  },
  {
    path: "/manifest",
    name: "Manifest",
    component: ManifestTab,
  },
  {
    path: "/files",
    name: "Files",
    component: FilesTab,
  },
  {
    path: "/dex",
    name: "Dex",
    component: DexTab,
  },
  {
    path: "/nsc",
    name: "NSC",
    component: NSCTab,
  },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

export default router;
