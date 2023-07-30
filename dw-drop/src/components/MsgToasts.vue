<template>
  <div
    class="toast-container position-fixed bottom-0 end-0 p-3"
    style="z-index: 2000"
  >
    <div
      v-for="toast in toasts"
      :key="toast.id"
      class="toast show"
      :class="{ toastfade: sticky != toast.id }"
      role="alert"
      aria-live="assertive"
      aria-atomic="true"
    >
      <div
        class="toast-header text-white"
        :class="{
          'bg-success': toast.type === 'success',
          'bg-secondary': toast.type === 'information',
          'bg-danger': toast.type === 'error',
        }"
      >
        <strong class="me-auto">
          <span v-if="toast.type === 'success'">{{ $t("toast.success") }}</span>
          <span v-else-if="toast.type === 'error'">{{
            $t("toast.error")
          }}</span>
          <span v-else>{{ $t("toast.information") }}</span>
        </strong>
        <button
          type="button"
          class="btn-close"
          data-bs-dismiss="toast"
          aria-label="Close"
          @click="removeToast(toast.id)"
        />
      </div>
      <div class="toast-body">
        {{ toast.message }}
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import Toast from "@/typings/toast";
import { useStore } from "@/store";

export default defineComponent({
  name: "MsgToasts",
  setup() {
    const store = useStore();
    /* eslint-disable no-unused-vars */
    const toasterSubscribe = (handler: (_: Toast) => void): void => {
      store.dispatch("global/toasterSubscribe", handler);
    };
    /* eslint-enable no-unused-vars */

    return { toasterSubscribe };
  },
  data() {
    return {
      count: 0,
      toasts: [] as Toast[],
      sticky: undefined as number | undefined,
    };
  },
  mounted() {
    this.toasterSubscribe(this.handleEvent);
  },
  methods: {
    handleEvent(payload: Toast): void {
      if (payload.message) {
        this.addToast(payload);
      }
    },
    addToast(toast: Toast) {
      // make sticky all error messages
      if (toast.type == "error") {
        toast.sticky = true;
      }

      // remove sticky if there was one
      if (this.sticky != undefined) {
        this.removeToast(this.sticky);
        this.sticky = undefined;
      }

      // register toast
      const id = this.count++;
      toast.id = id;
      this.toasts.push(toast);
      if (toast.sticky) {
        this.sticky = id;
      } else {
        setTimeout(() => {
          this.removeToast(id);
        }, 5000);
      }
    },
    removeToast(toast_id?: number) {
      if (toast_id == undefined) {
        return;
      }
      const index = this.toasts.findIndex((toast) => toast.id === toast_id);
      if (index !== -1) {
        this.toasts.splice(index, 1);
      }
    },
  },
});
</script>

<style scoped>
div.toastfade {
  animation: 5s ease 0s normal forwards 1 fade;
}

@keyframes fade {
  0% {
    opacity: 0;
  }
  7% {
    opacity: 1;
  }
  55% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
}
</style>
