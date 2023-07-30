## Installation

Library and tools are compiled at once using cargo:

```
$ cargo build --release --workspace
```

## Usage

The DroidWorks project includes a library, which can be used to build analysis
and apk crawling tools, and several tools that are build upon this library.

### Library

To build and open the documentation for all the crates included in the DroidWorks
library, simply run:

```
$ cargo doc --workspace --no-deps --open
```

Then, the droidworks crate can be imported in your project, and used after importing it with:

```
use droidworks::prelude::*;
```

### Tools

#### DroidWorks

The DroidWorks tools are accessible via different subcommands from the main
binary:

```
$ cargo run --release -- -h
```

##### List of subcommands

#### Summary

Here is a list of android application tools that are provided with droidworks:

 - `aresources` prints apk resources in aapt form.
 - `callgraph` generates dex code callgraph.
 - `dexdissect` dumps dex tables.
 - `disas` is objdump for dex bytecode, and can also generate control flow graphs.
 - `hierarchy` generates classes hierarchy graph.
 - `manifest` prints package manifest in classic xml format.
 - `nsc` prints apk network security configuration if there is.
 - `packageinfo` prints various information about the given file.
 - `permissions` allow listing and manipulation of permissions list in an apk.
 - `stats` prints basic statistics about found and missing classes.
 - `strip` build a callgraph and iteratively strip methods that could lookup unknown class, method or field.
 - `typecheck` runs a typechecking pass onto dalvik bytecode.

Moreover, it provides the two following utility commands:

 - `gen-completions` generates completions file for shells.
 - `help` prints the complete command help.

### Callgraph generation

```
$ cargo run --release -- <file> callgraph -r bytecode -o callgraph.dot
```

The generated dot file is generally too big to be processed by graphviz or other dot
viewing tool. To address this issue, filters can be applied at the object (the fully
qualified class name) and method level (name of the method solely, no argument),
independently. The parameter is a (single-line mode) regex whose syntax is described
[here](https://docs.rs/regex/1.5.4/regex/#syntax). The resulting callgraph then
contains all the paths of method calls leading to all the selected object methods.

For example:

```
$ cargo run --release -- <file> callgraph -r bytecode -o callgraph.dot \
      --target-object '^com/example/' \
      --target-method 'string$'
```
