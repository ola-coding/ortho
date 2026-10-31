import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        include: ['tests/**/*.test.ts'],
        // Many tests build a parser and link a whole example, several seconds
        // each once every file runs in parallel.
        testTimeout: 30_000
    }
});
