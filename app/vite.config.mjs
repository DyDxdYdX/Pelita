import { defineConfig } from 'vite';
import { dirname, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { readFileSync, readdirSync } from 'node:fs';

const require = createRequire(import.meta.url);
const pdfRoot = dirname(require.resolve('pdfjs-dist/package.json'));

const pdfResources = {
  name: 'pelita-pdf-resources',
  generateBundle() {
    for (const directory of ['cmaps', 'iccs', 'standard_fonts', 'wasm']) {
      for (const entry of readdirSync(resolve(pdfRoot, directory), { withFileTypes: true })) {
        if (!entry.isFile()) continue;
        this.emitFile({
          type: 'asset',
          fileName: `pdf/${directory}/${entry.name}`,
          source: readFileSync(resolve(pdfRoot, directory, entry.name)),
        });
      }
    }
    this.emitFile({
      type: 'asset',
      fileName: 'pdf/LICENSE',
      source: readFileSync(resolve(pdfRoot, 'LICENSE')),
    });
  },
};

export default defineConfig({
  base: './',
  clearScreen: false,
  plugins: [pdfResources],
  server: { port: 1420, strictPort: true },
  build: {
    rollupOptions: {
      input: {
        operator: resolve(import.meta.dirname, 'index.html'),
        projector: resolve(import.meta.dirname, 'projector.html'),
      },
    },
  },
});
