import tseslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import eslintConfigPrettier from "eslint-config-prettier";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default [
  {
    ignores: ["dist/**", "dist-frontend/**", "build/**", "node_modules/**", "src-tauri/**"]
  },
  {
    files: ["**/*.ts"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        project: ["./tsconfig.renderer.json"],
        tsconfigRootDir: __dirname
      }
    },
    plugins: {
      "@typescript-eslint": tseslint
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_" }],
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      eqeqeq: ["error", "always", { null: "ignore" }],
      "prefer-const": "error"
    }
  },
  eslintConfigPrettier
];
