# Contributing to LoopCode

Thanks for your interest in contributing.

## License

LoopCode is [Apache License 2.0](./LICENSE). By submitting a contribution, you
agree it is licensed under Apache-2.0 (see Apache-2.0 §5).

Do not introduce Category X dependencies (GPL/AGPL/LGPL, SSPL, field-of-use,
etc.) into distributed artifacts. The release SBOM gate enforces this
(`scripts/check-sbom-licenses.mjs`).

There is no CLA and no DCO sign-off requirement.

## Development

```bash
npm install
npm run lint
npm test
npm run tauri dev      # full app (Rust Core + WebView)
npm run tauri build   # production desktop bundle
npm run sbom          # CycloneDX SBOM → sbom/
npm run release-gate  # full RC validation gate
```

## Security

Report security issues privately — see [docs/SECURITY.md](./docs/SECURITY.md).
Do not file a public issue with exploit details.
