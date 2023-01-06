Utility tools for labelme json files

```console
lmrs <COMMAND>
```

# Commands

## swap
Add/Swap imagePath's prefix.
e.g. `"imagePath": "img.jpg"` -> `"imagePath": "../images/img.jpg"`

Can be useful when combined with labelme's --output option.

## svg
Create SVG image from labeme annotation.

## html
Create HTML with svgs from labelme directory.

## validate
Validate the number of annotations based on the given rules.

```console
lmrs validate app/tests/rules.txt app/tests --verbose
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
