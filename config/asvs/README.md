# OWASP ASVS requirements

| | |
|---|---|
| File | `asvs-5.0.0.flat.json` (unmodified) |
| Standard | OWASP Application Security Verification Standard 5.0.0 |
| Source | GitHub release `v5.0.0_release` of [OWASP/ASVS](https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release), asset `OWASP_Application_Security_Verification_Standard_5.0.0_en.flat.json` |
| Downloaded | 2026-11-12 |
| SHA-256 | `8201b20eec2908c3380ac600c91c8ba746346fbb808859366abb232027532311` |
| Requirements | 345 (level 1: 70, level 2: 183, level 3: 92) |
| License | [Creative Commons Attribution-ShareAlike 4.0](https://creativecommons.org/licenses/by-sa/4.0/) — © OWASP Foundation |

The file is embedded in the binary by `opentatu_core::AsvsCatalog::embedded` and is the only
source of requirement ids (maps, coverage). OpenTatu's own code stays under the MIT license;
this file keeps its CC BY-SA 4.0 license and is not modified. Which levels count as
applicable is the owner's decision Q-013 (`opentatu_core::APPLICABLE_LEVEL`: levels 1–3).

To change the ASVS version: add the new export next to this one, update this table and
`ASVS_VERSION`, and write an ADR (stored maps keep the version they were made with).
