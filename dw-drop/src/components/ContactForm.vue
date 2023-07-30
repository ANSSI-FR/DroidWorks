<template>
  <div
    id="contactForm"
    class="modal fade"
    data-bs-backdrop="true"
    data-bs-keyboard="true"
    tabindex="-1"
    aria-labelledby="backdropLabel"
    aria-hidden="true"
  >
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5
            id="backdropLabel"
            class="modal-title"
          >
            {{ $t("topmenu.contact") }}
          </h5>
          <button
            type="button"
            class="btn-close"
            data-bs-dismiss="modal"
            aria-label="Close"
          />
        </div>
        <div class="modal-body">
          <div class="mb-3">
            <p>
              {{ $t("contact.text") }}
              <b>droidworks@ssi.gouv.fr</b>
              .
            </p>
            <p style="white-space: nowrap">
              {{ $t("contact.version") }}
              <b>{{ version }}</b>
              .
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { app } from "@tauri-apps/api";

export default defineComponent({
  name: "ContactForm",
  data() {
    return {
      version: "undefined",
    };
  },
  mounted() {
    app
      .getVersion()
      .then((version) => {
        this.version = version;
      })
      .catch((error) => {
        console.error(error);
      });
  },
});
</script>
