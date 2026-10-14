import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);

async function systemFonts(): Promise<string[]> {
  const options = { encoding: 'utf8' as const, timeout: 15000, maxBuffer: 8 * 1024 * 1024 };
  let families: string[];
  if (process.platform === 'win32') {
    const { stdout } = await execute('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
      '[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new(); Add-Type -AssemblyName System.Drawing; $fonts = New-Object System.Drawing.Text.InstalledFontCollection; ConvertTo-Json -InputObject @($fonts.Families | ForEach-Object { $_.Name })'], options);
    families = JSON.parse(stdout);
  } else if (process.platform === 'darwin') {
    const { stdout } = await execute('system_profiler', ['SPFontsDataType', '-json'], options);
    families = [];
    const visit = (value: unknown) => {
      if (Array.isArray(value)) value.forEach(visit);
      else if (value && typeof value === 'object') {
        for (const [key, child] of Object.entries(value)) {
          if (key === 'family' && typeof child === 'string') families.push(child);
          else visit(child);
        }
      }
    };
    visit(JSON.parse(stdout));
  } else {
    const { stdout } = await execute('fc-list', ['--format', '%{family[0]}\n'], options);
    families = stdout.split('\n');
  }
  return [...new Set(families.map(family => family.trim()).filter(Boolean))].sort();
}

// Serve only font family names to the local development preview; desktop uses Rust.
function previewFonts(): Plugin {
  return {
    name: 'typelet-preview-fonts',
    configureServer(server) {
      server.middlewares.use('/__typelet/fonts', (request, response) => {
        if (request.method !== 'GET') { response.statusCode = 405; response.end(); return; }
        void systemFonts().then(families => {
          response.setHeader('Content-Type', 'application/json; charset=utf-8');
          response.setHeader('Cache-Control', 'no-store');
          response.end(JSON.stringify(families));
        }).catch(() => { response.statusCode = 503; response.end(); });
      });
    },
  };
}

export default defineConfig({
  base: './',
  plugins: [svelte(), previewFonts()],
});
