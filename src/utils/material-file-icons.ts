const iconAssets = import.meta.glob<string>(
  "../../node_modules/material-icon-theme/icons/{angular,audio,c,console,cpp,csharp,css,dart,database,docker,document,eslint,exe,file,font,folder,folder-open,folder-root,folder-root-open,folder-components,folder-components-open,folder-config,folder-config-open,folder-dist,folder-dist-open,folder-git,folder-git-open,folder-github,folder-github-open,folder-images,folder-images-open,folder-lib,folder-lib-open,folder-node,folder-node-open,folder-public,folder-public-open,folder-resource,folder-resource-open,folder-src,folder-src-open,folder-test,folder-test-open,folder-vscode,folder-vscode-open,git,go,html,image,java,javascript,json,key,kotlin,less,license,lock,markdown,nodejs,npm,pdf,php,powerpoint,powershell,prettier,python,react,react_ts,readme,ruby,rust,sass,settings,svelte,svg,swift,table,toml,typescript,typescript-def,video,vite,vue,word,xml,yaml,zip}.svg",
  { eager: true, import: "default", query: "?url" },
);

const ICON_BY_EXTENSION: Record<string, string> = {
  avi: "video",
  bmp: "image",
  bz2: "zip",
  c: "c",
  cc: "cpp",
  cjs: "javascript",
  cpp: "cpp",
  cs: "csharp",
  css: "css",
  csv: "table",
  dart: "dart",
  db: "database",
  doc: "word",
  docx: "word",
  eot: "font",
  gif: "image",
  go: "go",
  gz: "zip",
  h: "c",
  hpp: "cpp",
  html: "html",
  ico: "image",
  java: "java",
  jpeg: "image",
  jpg: "image",
  js: "javascript",
  json: "json",
  jsonc: "json",
  jsx: "react",
  kt: "kotlin",
  less: "less",
  lock: "lock",
  md: "markdown",
  mjs: "javascript",
  mov: "video",
  mp3: "audio",
  mp4: "video",
  ogg: "audio",
  otf: "font",
  pdf: "pdf",
  php: "php",
  png: "image",
  ppt: "powerpoint",
  pptx: "powerpoint",
  ps1: "powershell",
  py: "python",
  rar: "zip",
  rb: "ruby",
  rs: "rust",
  sass: "sass",
  scss: "sass",
  sh: "console",
  sql: "database",
  svelte: "svelte",
  svg: "svg",
  swift: "swift",
  tar: "zip",
  toml: "toml",
  ts: "typescript",
  tsx: "react_ts",
  tsv: "table",
  ttf: "font",
  txt: "document",
  vue: "vue",
  wav: "audio",
  webm: "video",
  webp: "image",
  woff: "font",
  woff2: "font",
  xls: "table",
  xlsx: "table",
  xml: "xml",
  yaml: "yaml",
  yml: "yaml",
  zip: "zip",
};

const ICON_BY_NAME: Record<string, string> = {
  ".dockerignore": "docker",
  ".editorconfig": "settings",
  ".env": "key",
  ".gitattributes": "git",
  ".gitignore": "git",
  ".npmrc": "npm",
  ".prettierignore": "prettier",
  ".prettierrc": "prettier",
  "cargo.lock": "rust",
  "cargo.toml": "rust",
  dockerfile: "docker",
  license: "license",
  "license.md": "license",
  "package-lock.json": "npm",
  "package.json": "nodejs",
  "pnpm-lock.yaml": "nodejs",
  readme: "readme",
  "readme.md": "readme",
  "tsconfig.json": "typescript",
  "vite.config.js": "vite",
  "vite.config.ts": "vite",
  "yarn.lock": "nodejs",
};

const FOLDER_BY_NAME: Record<string, string> = {
  ".git": "folder-git",
  ".github": "folder-github",
  ".vscode": "folder-vscode",
  __tests__: "folder-test",
  assets: "folder-images",
  build: "folder-dist",
  components: "folder-components",
  config: "folder-config",
  dist: "folder-dist",
  images: "folder-images",
  img: "folder-images",
  lib: "folder-lib",
  libs: "folder-lib",
  node_modules: "folder-node",
  out: "folder-dist",
  public: "folder-public",
  resources: "folder-resource",
  src: "folder-src",
  static: "folder-public",
  test: "folder-test",
  tests: "folder-test",
};

function iconUrl(iconName: string) {
  return iconAssets[`../../node_modules/material-icon-theme/icons/${iconName}.svg`];
}

export function materialFileIcon(name: string) {
  const normalized = name.toLowerCase();
  const namedIcon = ICON_BY_NAME[normalized];
  if (namedIcon) return iconUrl(namedIcon);

  if (normalized.endsWith(".d.ts")) return iconUrl("typescript-def");
  const extension = normalized.includes(".") ? (normalized.split(".").pop() ?? "") : "";
  return iconUrl(ICON_BY_EXTENSION[extension] ?? "file");
}

export function materialFolderIcon(name: string, expanded: boolean, root = false) {
  if (root) return iconUrl(expanded ? "folder-root-open" : "folder-root");
  const baseIcon = FOLDER_BY_NAME[name.toLowerCase()] ?? "folder";
  return iconUrl(expanded ? `${baseIcon}-open` : baseIcon);
}
