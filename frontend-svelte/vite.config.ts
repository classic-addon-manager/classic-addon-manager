// @ts-ignore
import path from 'path';
import {defineConfig} from 'vite'
import {svelte} from '@sveltejs/vite-plugin-svelte'

// https://vitejs.dev/config/
export default defineConfig(({mode}) => ({
    plugins: [svelte()],
    resolve: {
        conditions: mode === 'test' ? ['browser'] : [],
        alias: {
            $lib: path.resolve("./src/lib"),
            $stores: path.resolve("./src/stores"),
            $screens: path.resolve("./src/screens"),
        },
    }
}));
