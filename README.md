Utility tools for labelme json files

## lm_swap_prefix
Add/Swap imagePath's prefix.
e.g. `"imagePath": "img.jpg"` -> `"imagePath": "../images/img.jpg"`

Can be useful when combined with labelme's --output option.

## lm2svg
Create SVG image from labeme annotation.

## lms2html
Create HTML with svgs from labelme directory.

## lm_validate
Validate the number of annotations based on the given rules.

```console
lm_validate app/tests/rules.txt app/tests --verbose
```