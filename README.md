Utility tools for labelme json files

# Install
Download pre-built binary from [Releases](https://github.com/ykszk/labelme-rs/releases) page.

Or compile from source:
```bash
cargo install --git https://github.com/ykszk/labelme-rs
```

## Auto-completion
For fish shell:
```bash
lmrs complete fish > ~/.config/fish/completions/lmrs.fish
```
## Man files
```bash
cargo xtask ~/.local/share/man/man1/
```

## Agent skill
An agent skill that teaches Claude Code how to use `lmrs` is available as a plugin.
Install it in Claude Code:
```
/plugin install lmrs --marketplace ykszk/labelme-rs
```
On older versions of Claude Code:
```
/plugin marketplace add ykszk/labelme-rs
/plugin install lmrs@labelme-rs
```
Run `/plugin marketplace update labelme-rs` to get the latest skill.

The skill itself is [plugin/skills/lmrs/SKILL.md](plugin/skills/lmrs/SKILL.md) and can be copied to other agents supporting Agent Skills.


# Usage

Invoke commands like so:
```bash
lmrs <COMMAND> [args and options]
```

Use
```bash
lmrs <COMMAND> --help
```
to see help in full detail.

# Generic commands
Commands that work for any json.

## ndjson
Create ndjson/json file from the given json-containing directory.
Resulting lines are structured as `{content:JSON_CONTENT, filename:JSON_FILENAME}`.
Use `split` command to undo.

```bash
lmrs ndjson JSON_DIRECTORY > jsons.ndjson
```

## split
Undo `lmrs ndjson`.
i.e. split ndjson file into separate json files using `filename` values as filenames.

Simple use:
```bash
lmrs split data.ndjson -o outdir
```

Use with `jq` filtering:
```bash
lmrs ndjson json_indir | jq -c 'select(.is_good)' | lmrs split -o json_outdir
```

## drop
Drop duplicates except for the first occurrence

```bash
cat 1.ndjson 2.ndjson | lmrs drop --key id
```

## join
Join (not concatenate) ndjson files

```bash
lmrs join left.ndjson right.ndjson
```

# Labelme commands
Commands that only work for json in labelme format.

## swap
Add/Swap imagePath's prefix.
e.g. `"imagePath": "img.jpg"` -> `"imagePath": "../images/img.jpg"`


Changing parent directory of the image (json files are updated in place):
```bash
lmrs swap --prefix "../images" JSON_DIR
```

Changing image extension:
```bash
lmrs swap --suffix ".png" JSON_DIR
```

Writing to another directory or piping ndjson:
```bash
lmrs swap --prefix "../images" JSON_DIR OUTPUT_DIR
lmrs ndjson JSON_DIR | lmrs swap --prefix "../images" > swapped.ndjson
```

Can be useful when combined with labelme's --output option.

## filter
Filter valid/invalid data. See `validate` command for validation details.

```bash
lmrs ndjson lmrs/tests | lmrs filter - -r lmrs/tests/rules.txt
```

## remove
Remove annotation with specified label(s).

```bash
lmrs ndjson lmrs/tests | lmrs remove - --label TL
```

## svg
Create SVG image from labelme annotation.

```bash
lmrs svg labelme.json labelme.svg
```

## catalog
Create HTML cataloging svgs from labelme directory.

```bash
lmrs catalog json_directory catalog.html
lmrs ndjson json_directory | jq -c 'select(.content.flags.good)' | lmrs catalog - catalog.html
```

## validate
Validate the number of points in annotations based on the given rules and show the list of complaints about the annotation.

```bash
lmrs validate lmrs/tests/rules.txt lmrs/tests --verbose
```

Output:
```
img1.json,Unsatisfied rules; "TR > 0": 0 vs. 0,  "BL > 0": 0 vs. 0,  "BR > 0": 0 vs. 0,  "TL == TR": 1 vs. 0
```

Rule example:
```
TL > 0
TR > 0
BL > 0
BR > 0
TL == TR
BL == BR
```

## shapeshift
Change shape type (`c2p`: circle to point, `p2p`: polygon to point).

```bash
lmrs ndjson json_directory | lmrs shapeshift - c2p
```

## merge
Merge shapes of labelme annotations sharing the same `filename`

```bash
lmrs merge annotator1.ndjson annotator2.ndjson > merged.ndjson
```

## mat
Apply 3x3 transformation matrix (or translation/scale/rotation) to the points

```bash
lmrs mat labelme.json transformed.json --scale 0.5 0.5
lmrs ndjson json_directory | lmrs mat --ndjson - --translate 10 20
```

## resize
Scale point coordinates according to the resize parameter

```bash
lmrs ndjson . | lmrs resize - 50%
```

## init
Create empty labelme json for the image

```bash
lmrs init image_directory | lmrs split -o json_directory
```

## exist
Check `imagePath` existence

Print existing image files:
```bash
lmrs exist labelmes.ndjson | jq -cr ".content.imagePath"
```

Print missing image files:
```bash
lmrs exist labelmes.ndjson --invert | jq -cr ".content.imagePath"
```

## archive
Archive jsons and associated image files as tar

```bash
lmrs archive json_directory | zstd > archived_json_and_image.tar.zst
```

## count
Count flags

```bash
lmrs count json_directory
```

## stats
Count shapes per shape type and label

```bash
lmrs ndjson json_directory | lmrs stats
```

## prune
Prune shapes outside image boundaries

```bash
lmrs ndjson json_directory | lmrs prune --mode majority
```

## sort
Sort shapes by point coordinates

```bash
lmrs sort labelme.json --by-x
lmrs ndjson json_directory | lmrs sort - --labels TL,TR
```

## browse
Browse annotations

Start browser:
```bash
lmrs browse <json file or directory> --open
```
Specifying:
- json file will open the file
- directory will open index page linking to each annotation

- Add `--address 0.0.0.0` option to allow access from other hosts.
- Add `--port 0` to use a new port assigned by OS.

Generate default config
```bash
lmrs browse - --default
```

# Python binding
Install:

```bash
cd lmrspy
maturin develop --release
python -m unittest discover -v tests/
```

Example:

```python
import lmrspy
rules = ['TL==1', 'TL>0']
flags = ['jsons', 'containing', 'flags', 'will', 'be', 'validated']
ignores = ['flags', 'to', 'ignore', 'json']
validator = lmrspy.Validator(rules, flags, ignores)
validator.validate_json_file('labelme.json')
# true if valid and false if skipped
# raises exception for invalid data
```
