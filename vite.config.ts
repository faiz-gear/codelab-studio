import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const REACT_VENDOR_PATTERN = /node_modules[\\/](react|react-dom|scheduler)[\\/]/;

function resolveManualChunk(id: string): string | undefined {
  if (REACT_VENDOR_PATTERN.test(id)) {
    return "vendor-react";
  }

  return undefined;
}

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: resolveManualChunk,
      },
    },
  },
});
