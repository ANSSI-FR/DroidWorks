<template>
  <div
    id="apkSignerModal"
    class="modal fade"
    data-bs-backdrop="static"
    data-bs-keyboard="false"
    tabindex="-1"
    aria-labelledby="staticBackdropLabel"
    aria-hidden="true"
  >
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5
            id="staticBackdropLabel"
            class="modal-title"
          >
            {{ $t("apksign.signing") }}
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
            <label
              class="form-label"
              for="applicationPath"
            >{{
              $t("apksign.application")
            }}</label>
            <div class="input-group">
              <input
                id="applicationPath"
                v-model="applicationPath"
                type="text"
                class="form-control"
                :placeholder="$t('apksign.application.path')"
              >
              <button
                id="applicationPathButton"
                class="btn btn-outline-secondary"
                type="button"
                @click="selectApplicationFile"
              >
                ...
              </button>
            </div>
          </div>

          <div class="form-check">
            <input
              id="keyMethod1"
              v-model="signer"
              class="form-check-input"
              type="radio"
              name="keyMethod"
              value="keystore"
              checked
            >
            <div class="mb-3">
              <label
                class="form-label"
                for="keystorePath"
              >{{
                $t("apksign.keystore")
              }}</label>
              <div class="input-group">
                <input
                  id="keystorePath"
                  v-model="keystorePath"
                  type="text"
                  class="form-control"
                  :placeholder="$t('apksign.keystore.path')"
                  :disabled="signer !== 'keystore'"
                >
                <button
                  id="keystorePathButton"
                  class="btn btn-outline-secondary"
                  type="button"
                  :disabled="signer !== 'keystore'"
                  @click="selectKeystoreFile"
                >
                  ...
                </button>
              </div>
              <div class="input-group">
                <input
                  id="keystorePass"
                  v-model="keystorePass"
                  type="password"
                  class="form-control"
                  :placeholder="$t('apksign.keystore.pass')"
                  :disabled="signer != 'keystore'"
                >
              </div>
            </div>
          </div>

          <div class="form-check">
            <input
              id="keyMethod2"
              v-model="signer"
              class="form-check-input"
              type="radio"
              name="keyMethod"
              value="keyAndCert"
            >
            <label
              class="form-check-label"
              for="keyMethod2"
            >
              {{ $t("apksign.keycert") }}
            </label>
            <div class="input-group mt-1">
              <input
                id="keyPath"
                v-model="keyPath"
                type="text"
                class="form-control"
                :placeholder="$t('apksign.keycert.keypath')"
                :disabled="signer !== 'keyAndCert'"
              >
              <button
                id="keyPathButton"
                class="btn btn-outline-secondary"
                type="button"
                :disabled="signer !== 'keyAndCert'"
                @click="selectKeyFile"
              >
                ...
              </button>
            </div>
            <div class="input-group">
              <input
                id="keyPass"
                v-model="keyPass"
                type="password"
                class="form-control"
                :placeholder="$t('apksign.keycert.keypass')"
                :disabled="signer != 'keyAndCert'"
              >
            </div>
            <div class="input-group">
              <input
                id="certPath"
                v-model="certPath"
                type="text"
                class="form-control"
                :placeholder="$t('apksign.keycert.certpath')"
                :disabled="signer !== 'keyAndCert'"
              >
              <button
                id="certPathButton"
                class="btn btn-outline-secondary"
                type="button"
                :disabled="signer !== 'keyAndCert'"
                @click="selectCertFile"
              >
                ...
              </button>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button
            type="button"
            class="btn btn-secondary"
            data-bs-dismiss="modal"
          >
            {{ $t("apksign.close") }}
          </button>
          <button
            type="button"
            class="btn btn-primary"
            :disabled="applicationPath === ''"
            @click="sign"
          >
            {{ $t("apksign.sign") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { dialog, tauri } from "@tauri-apps/api";
import Toast from "@/typings/toast";
import { useStore } from "@/store";

export default defineComponent({
  name: "ApkSignerModal",
  setup() {
    const store = useStore();
    const toast = (toast: Toast): void => {
      store.dispatch("global/toast", toast);
    };

    return { toast };
  },
  data() {
    return {
      applicationPath: "",
      signer: "keystore",
      keystorePath: "",
      keystorePass: "",
      keyPath: "",
      keyPass: "",
      certPath: "",
    };
  },
  methods: {
    selectApplicationFile() {
      dialog
        .open({
          filters: [
            {
              name: this.$t("apksign.app") + " (*.apk)",
              extensions: ["apk"],
            },
            {
              name: this.$t("files.all") + " (*)",
              extensions: ["*"],
            },
          ],
          multiple: false,
          directory: false,
        })
        .then((result) => {
          // result should be string and not string[] due to 'multiple: false' flag.
          this.applicationPath = result as string;
        })
        .catch((error) => {
          console.error(error);
          this.toast({ type: "error", message: error });
        });
    },
    selectKeystoreFile() {
      dialog
        .open({
          filters: [
            {
              name:
                this.$t("apksign.keystore") +
                " (*.keystore, *.ks, *.jks, *.pkcs12, *.pk12)",
              extensions: ["keystore", "ks", "jks", "pkcs12", "pk12"],
            },
            {
              name: this.$t("files.all") + " (*)",
              extensions: ["*"],
            },
          ],
          multiple: false,
          directory: false,
        })
        .then((result) => {
          // result should be string and not string[] due to 'multiple: false' flag.
          this.keystorePath = result as string;
        })
        .catch((error) => {
          console.error(error);
          this.toast({ type: "error", message: error });
        });
    },
    selectKeyFile() {
      dialog
        .open({
          filters: [
            {
              name: this.$t("apksign.key") + " (*.pk8)",
              extensions: ["pk8"],
            },
            {
              name: this.$t("files.all") + " (*)",
              extensions: ["*"],
            },
          ],
          multiple: false,
          directory: false,
        })
        .then((result) => {
          // result should be string and not string[] due to 'multiple: false' flag.
          this.keyPath = result as string;
        })
        .catch((error) => {
          console.error(error);
          this.toast({ type: "error", message: error });
        });
    },
    selectCertFile() {
      dialog
        .open({
          filters: [
            {
              name: this.$t("apksign.certificate") + " (*.pem)",
              extensions: ["pem"],
            },
            {
              name: this.$t("files.all") + " (*)",
              extensions: ["*"],
            },
          ],
          multiple: false,
          directory: false,
        })
        .then((result) => {
          // result should be string and not string[] due to 'multiple: false' flag.
          this.certPath = result as string;
        })
        .catch((error) => {
          console.error(error);
          this.toast({ type: "error", message: error });
        });
    },
    sign() {
      if (this.signer == "keystore") {
        const signing = {
          applicationPath: this.applicationPath,
          signer: {
            keystore: {
              ks_path: this.keystorePath,
              ks_pass: this.keystorePass,
            },
          },
        };
        console.log(signing);
        this.toast({
          type: "information",
          message: this.$t("apksign.signing.app", {
            filename: signing.applicationPath,
          }),
          sticky: true,
        });
        tauri
          .invoke<string>("apksigner", { signing })
          .then((newFileName) => {
            console.log(newFileName);
            this.toast({
              type: "success",
              message: this.$t("apksign.signed.app", {
                filename: newFileName,
              }),
            });
          })
          .catch((error) => {
            console.error(error);
            this.toast({ type: "error", message: error });
          });
      } else if (this.signer == "keyAndCert") {
        const signing = {
          applicationPath: this.applicationPath,
          signer: {
            keyAndCert: {
              key_path: this.keyPath,
              key_pass: this.keyPass,
              cert_path: this.certPath,
            },
          },
        };
        console.log(signing);
        tauri
          .invoke<string>("apksigner", { signing })
          .then((newFileName) => {
            console.log(newFileName);
            this.toast({
              type: "success",
              message: this.$t("apksign.signed.app", {
                filename: newFileName,
              }),
            });
          })
          .catch((error) => {
            console.error(error);
            this.toast({ type: "error", message: error });
          });
      } else {
        console.error("signer must be 'keyMethod' or 'keyAndCert'");
        this.toast({
          type: "error",
          message: "signer must be 'keyMethod' or 'keyAndCert'",
        });
        return;
      }
    },
  },
});
</script>
