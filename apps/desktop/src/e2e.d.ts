interface ImportMetaEnv {
  readonly VITE_OSMIUM_E2E?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

interface Window {
  osmiumE2eSourceDirectory?: string;
  osmiumE2eNewSourceDirectory?: string;
  osmiumE2eDistributionPath?: string;
  osmiumE2eExportDestination?: string;
  osmiumE2eConfirmRemoval?: boolean;
}
