# Third-party notices

This distribution carries files from other projects. This notice names each one, the terms it travels under, and the exact revision it was taken from. `sdd license --third-party` prints this notice from the binary.

## SimpleEnglish

spec-driven-docs adopts the SimpleEnglish writing pattern and vendors its consumed surface byte-for-byte for offline use. The `Plain` mode is active by default for in-scope technical writing.

- Project: SimpleEnglish
- Repository: https://github.com/AminBlg/SimpleEnglish
- Adopted version: v2.0.0
- Resolved object ID: d9e523409686e88df175623f7a692d025aff95b1
- License: MIT

The vendored files live under `third-party/simpleenglish/`. `third-party/simpleenglish/UPSTREAM.json` records every vendored path with a SHA-256 digest over its bytes. The vendored paths are:

- `LICENSE`
- `prompts/system-prompt.md`
- `skills/simple-english/SKILL.md`
- `skills/simple-english/references/checklist.md`
- `skills/simple-english/references/strict-vocabulary.md`
- `skills/simple-english/references/use-cases.md`
- `skills/simple-english/references/word-swaps.md`
- `hooks/hooks.json`
- `src/hooks/README.md`
- `src/hooks/package.json`
- `src/hooks/simple-english-activate.js`
- `src/hooks/simple-english-activate.test.js`
- `src/hooks/lint_hook.py`
- `src/hooks/test_lint_hook.py`
- `evals/ste_lint.py`
- `evals/slop.tsv`

### SimpleEnglish MIT license

```text
MIT License

Copyright (c) 2026 AminBlg

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## ASD-STE100

SimpleEnglish takes its structural rules from ASD-STE100 Simplified Technical English. ASD-STE100 is a standard of the AeroSpace and Defence Industries Association of Europe (ASD), maintained by the Simplified Technical English Maintenance Group (STEMG). spec-driven-docs adopts the SimpleEnglish pattern that draws on that standard. It does not imply endorsement or certification by ASD, STEMG, or the SimpleEnglish maintainers. No tool can guarantee ASD-STE100 compliance. The official standard is a free download at asd-ste100.org.
