# Rawrchive

Pure safe Rust RAR5 extractor. No unRAR code!

## Provenance

Rawrchive contains no code from RARLAB's unRAR utility, making it entirely free software!

* The archive format is implemented based on RARLAB's official public [technical note](https://www.rarlab.com/technote.htm).
* The decompression code is ported from [rardecode](https://github.com/nwaples/rardecode), an independent Go implementation by Nicholas Waples, used under its BSD 2-Clause license (see `LICENSE-rardecode.md`).

## License

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

See [LICENSE.md](LICENSE.md) for the full text.
