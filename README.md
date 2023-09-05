Utility tools for labelme json files

# Install
Download pre-built binary from [Releases](https://github.com/ykszk/labelme-rs/releases) page.

Or compile from the source code:
```console
cargo install --git https://github.com/ykszk/labelme-rs
```

# Usage

Invoke commands like so:
```console
lmrs <COMMAND> [args and options]
```

# Commands
Use
```console
lmrs <COMMAND> --help
```
to see help in full detail.

## jsonl
Create jsonl/ndjson file from the given json-containing directory.
`filename` key is added to each json to make this process invertible.
Use `split` command to invert.

```console
lmrs jsonl json_directory/ > jsons.jsonl
```

## split
Invert `lmrs jsonl` process.
i.e. split jsonl file into separate json files using `filename` values as filenames.

Simple use:
```console
lmrs data.jsonl -o outdir
```

Use with `jq` filtering:
```console
lmrs jsonl json_indir | jq -c 'select(.is_good)' | lmrs split -o json_outdir
```

## filter
Filter valid/invalid data. See `validate` command for validation details.

```console
lmrs jsonl app/tests | lmrs filter - -r app/tests/rules.txt
```

## swap
Add/Swap imagePath's prefix.
e.g. `"imagePath": "img.jpg"` -> `"imagePath": "../images/img.jpg"`

Can be useful when combined with labelme's --output option.

## svg
Create SVG image from labeme annotation.

## html
Create HTML with svgs from labelme directory.

## validate
Validate the number of points in annotations based on the given rules and show the list of complaints about the annotation.

```console
lmrs validate app/tests/rules.txt app/tests --verbose
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

## drop
Drop duplicates except for the first occurrence

```console
cat 1.ndjson 2.ndjson | lmrs drop --key id
```

# Python binding

```console
cd lmrspy
maturin develop --release
```

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
