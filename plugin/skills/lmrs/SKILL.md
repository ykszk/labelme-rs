---
name: lmrs
description: Inspect, validate, filter and transform labelme annotation JSON files with the `lmrs` CLI. Use when working with labelme .json annotations or ndjson built from them, e.g. counting labels or flags, checking point-count rules, finding missing images, swapping imagePath, resizing, pruning, sorting, merging, or rendering annotations to SVG/HTML.
---

# lmrs

`lmrs` is a CLI for labelme json files (https://github.com/ykszk/labelme-rs).
Run `lmrs <COMMAND> --help` for exact arguments before using a command for the first time in a session; options change between versions.

If `lmrs` is not installed, tell the user and point them to the Releases page or `cargo install --git https://github.com/ykszk/labelme-rs`.

## Core workflow: ndjson pipelines

Most commands read ndjson from stdin (`-`, usually the default) and write ndjson to stdout.
Each line is `{"content": <labelme json>, "filename": "<json path>"}`.

1. `lmrs ndjson DIR` turns a directory of json files into ndjson (`-g '**/*.json'` for recursive search)
2. Pipe it through lmrs commands, or through `jq -c` for custom filtering
3. `lmrs split -o OUTDIR` writes the result back to separate json files

```bash
lmrs ndjson anns | jq -c 'select(.content.flags.good)' | lmrs remove - --label TL | lmrs split -o out
```

`jq` works on the same lines, e.g. `jq -r .filename` to list files or `jq -c .content` to get the bare labelme json.

## Commands

Inspect:
- `stats`: shape counts per shape type and label (`lmrs ndjson anns | lmrs stats`)
- `count`: flag counts (`lmrs count anns`)
- `exist`: entries whose `imagePath` exists; add `--invert` for missing images
- `validate RULES DIR`: list annotations violating point-count rules (`--verbose` for details, `--stats` for a summary)

Filter and edit:
- `filter -r rules.txt` (`--invert` for invalid lines)
- `remove --label L` / `remove --shape TYPE` (`--invert` to keep only those)
- `prune --mode any|majority|all`: remove shapes outside the image
- `sort`: sort shapes by y (`--by-x`, `--descending`, `--labels A,B`)
- `shapeshift - c2p|p2p`: circle or polygon to point
- `drop --key KEY`: drop duplicates, keeping the first
- `join A B --key filename --mode inner|left|outer`: join ndjson files
- `merge A B`: merge shapes of annotations with the same filename

Geometry and paths:
- `resize - 50%`: scale points (imagemagick-like size; `--image DIR` also writes resized images)
- `mat --scale SX SY` / `--translate TX TY` / `--rotate DEG` / `--matrix ...` (`--ndjson` for ndjson input)
- `swap --prefix DIR` / `swap --suffix .png`: rewrite `imagePath`

Create and output:
- `init IMAGE_DIR`: empty labelme ndjson for images (pipe to `split`)
- `svg JSON OUT.svg`, `catalog DIR_OR_NDJSON OUT.html`: render annotations (the images must exist)
- `archive DIR > out.tar`: tar of json files plus their images
- `browse DIR --open`: local web viewer (long-running server; start it only if the user asks)

## Validation rules

One rule per line, comparing label counts:

```
TL > 0
TL == TR
BL == BR
```

Use them with `validate` (report) or `filter -r` (keep valid lines).

## Cautions

- `swap` on a directory or single json **overwrites the input files** when OUTPUT is omitted. Preview a single file with `lmrs swap --prefix X file.json -`, or work on ndjson and write to a new directory.
- `split` does not overwrite existing files unless `--overwrite` is given.
- Paths in `imagePath` are resolved relative to the json/ndjson file; with stdin input, relative to the current directory.
- Leave the user's original annotations untouched unless they ask for in-place changes: write results to a new file or directory.
