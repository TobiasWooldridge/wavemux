# This repository is archived

`wavemux` now lives inside the SDR monorepo:

**https://github.com/TobiasWooldridge/sdr — `wavemux/`**

It moved there in the wavestack collapse (TobiasWooldridge/sdr#2246), which
replaced a cluster of tightly-coupled repos with one repo containing the same
directories. Every commit in this repository's history was carried across with
its paths rewritten, so `git log -- wavemux/` and `git blame` in the monorepo
reach back to the first commit made here.

## What this means

- **New work goes to the monorepo.** This repo is read-only.
- **Open issues moved.** GitHub preserves the old numbers as redirects.
- **Old links still resolve.** Commit SHAs cited in issues and PRs still point
  at this repository, which is why it is archived rather than deleted.

## Getting the code

```bash
git clone git@github.com:TobiasWooldridge/sdr.git
# wavemux/ is a real directory — no submodule init required
```
