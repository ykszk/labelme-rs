Utility tools for labelme json files

# Install
Download pre-built binary from [Releases](https://github.com/ykszk/labelme-rs/releases) page.

Or compile from source:
```console
cargo install --git https://github.com/ykszk/labelme-rs
```

## Auto-completion
For fish shell:
```console
cargo xtask complete fish -install
```

# Usage

Invoke commands like so:
```console
lmrs <COMMAND> [args and options]
```

Use
```console
lmrs <COMMAND> --help
```
to see help in full detail.

# Generic commands
Commands that work for any json.

## ndjson
Create ndjson/json file from the given json-containing directory.
Resulting lines are structured as `{content:JSON_CONTENT, filename:JSON_FILENAME}`.
Use `split` command to undo.

```console
lmrs ndjson JSON_DIRECTORY > jsons.ndjson
```

## split
Undo `lmrs ndjson`.
i.e. split ndjson file into separate json files using `filename` values as filenames.

Simple use:
```console
lmrs data.ndjson -o outdir
```

Use with `jq` filtering:
```console
lmrs ndjson json_indir | jq -c 'select(.is_good)' | lmrs split -o json_outdir
```

## drop
Drop duplicates except for the first occurrence

```console
cat 1.ndjson 2.ndjson | lmrs drop --key id
```

## join
Join (not concatenate) ndjson files

```console
lmrs join left.ndjson right.ndjson
```

# Labelme commands
Commands that only work for json in labelme format.

## swap
Add/Swap imagePath's prefix.
e.g. `"imagePath": "img.jpg"` -> `"imagePath": "../images/img.jpg"`


Changing pagent directory of the image
```console
lmrs swap JSON_DIR "../images"
```

Changing image extension:
```console
lmrs swap JSON_DIR "png" --suffix
```

Can be useful when combined with labelme's --output option.

## filter
Filter valid/invalid data. See `validate` command for validation details.

```console
lmrs ndjson lmrs/tests | lmrs filter - -r lmrs/tests/rules.txt
```

## remove
Remove annotation with specified label(s).

```console
lmrs ndjson lmrs/tests | lmrs remove - --labels TL
```

## svg
Create SVG image from labeme annotation.

## catalog
Create HTML cataloging svgs from labelme directory.

## validate
Validate the number of points in annotations based on the given rules and show the list of complaints about the annotation.

```console
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

## resize
Scale point coordinates according to the resize parameter

```console
lmrs ndjson . | lmrs resize - 50%
```

## init
Create empty labelme json for the image

```console
lmrs init image_directory | lmrs split -o json_directory
```

## exist
Check `imagePath` existence

Print missing image file:
```console
lmrs exist labelmes.ndjson | jq -cr ".content.imagePath"
```

# Python binding
Install:

```console
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
