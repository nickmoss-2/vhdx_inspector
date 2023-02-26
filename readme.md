# VHDX Inspector

## Overview
VHDX Inspector reads the header and metadata tables of a VHDX file and dumps
the details to the console. The text is generally readable but should be read
with consultation to Microsoft's published VHDX specification at
[[MS-VHDX]: Virtual Hard Disk v2 (VHDX) File Format](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-vhdx/83e061f8-f6e2-4de1-91bd-5d518a43d477).
This program understands version 7.0 of the specification.

## Usage
vhdx_inspector [args] \<file name\>

### -h, --help
Print this help message and exit immediately.

### -f, --follow
If the VHDX file is a differencing disk, print the parent disk's information and so on up the chain.

### -b, --blocks
Print the full block status information.

## License
VHDX Inspector is provided under the terms of the MIT license.