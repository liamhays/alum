# alum
Alum is a file transfer tool for HP's RPL calculators.

# Why Alum?
Alum (pronounced like the first part of "aluminum") is a command-line
application for transferring files to and from your calculator.

Alum is intended as a replacement for software like [the HP
Connectivity Kit](https://www.hpcalc.org/details/5890). Alum is better
because:

- It is cross-platform, currently tested on Windows and Linux
- It is programmed in Rust (compiles to an executable) and requires no
  external dependencies, except for one small package on Linux
- Alum uses Rust implementations of XModem and Kermit
  
I am retiring my previous software HPex, a GUI tool that accomplished
a similar task. HPex was written in Python 3 and wxPython, which meant
that the end user needed hundreds of megabytes of shared libraries and
executables to run HPex. Even worse, HPex wasn't exactly a great
application, and had no hope of being useful on Windows because of
various multi-platform limitations.

Alum is *not* a GUI because a) GUI programming gets in the way of good
development, b) in my experience, the average HP calculator user is
comfortable in the command line, and c) as mentioned above, the
software dependencies for GUI libraries are big and bloated. Alum is
fast, light, and small.

# Features
Alum can:

- Automatically detect a serial port to use
- Transfer via Kermit and XModem, both direct and to a server
- Calculate the checksum and size of any HP 48 object

# Usage
Download a binary from the Releases page and place it somewhere
convenient. You may wish to add it to your `PATH` variable. On Linux,
Alum requires `libudev` to be installed for serial port detection. It
is in every distro's package manager and is likely
pre-installed. On Windows, Alum requires no extra dependencies.

For full usage information, run Alum with no arguments. It includes
its own help, and this section is just a basic usage rundown.

Alum supports sending and receiving files to and from 128-byte `XRECV`
and `XSEND`, as well as full XModem server send and receive
functionality. The XModem server is integrated into the HP 49 series,
accessible via `[right-shift][right arrow]`, and is available as a
library for the HP 48 series at
[https://www.hpcalc.org/details/5412](https://www.hpcalc.org/details/5412).

Alum also supports sending files only (no receive) to the HP 48 Kermit
server.

## CLI
Alum uses a "subcommand" structure. The commands are:

- `xsend`: send file to XModem server
- `xget`: get file from XModem server
- `ksend`: send file to Kermit
- `info`: calculate file size and HP checksum on file

Each subcommand takes a file argument and optionally flags. Alum
contains help for each command---simply run the command with no
arguments (`alum xsend`) to get help.

## XModem transfers
By default, the `xsend` and `xget` commands transfer to and from the
XModem server. To communicate with `XSEND` and `XRECV`, specify the
`-d` (direct) flag, as in `alum xsend -d Arkalite.lib`.

For example, say we want to send the excellent game
[Arkalite](https://www.hpcalc.org/details/460) to the XModem
server. That is as simple as running:

```
$ alum xsend Arkalite.lib
Sending "Arkalite.lib" to XModem server...
################################################################################################  7/7  packets (100%)
Done!
File info:
  ROM Revision: X, Object CRC: #44ABh, Object length (bytes): 1776.0
```

In this example, Alum found the one physical serial port on the system
and used it automatically.

## Extra transfer features
To finish or close any server after a transfer, pass the `-f` flag to
Alum, like this: `alum -f ksend Arkalite.lib`. If the file transfer is
successful, Alum will send a quit command to the calculator after the
transfer.

# Limitations
Alum has only been tested with an HP 48GX. In addition, Alum will not
calculate the checksum of an HP 49 object, because I have found that
the algorithm I use for HP 48 objects doesn't work for HP 49
objects. Alum also does not currently support the 1K CRC direct
`XRECV` and `XSEND` added in the HP 49.

Alum also does not currently support receiving files over Kermit. I'm
working on it.

## Future features
- [ ] Kermit receive
- [ ] XModem server and Kermit server file listing
- [ ] 1K CRC direct XModem
- [ ] HP 49 object info


## XModem caveat
XModem is an old standard, and is so simple as to be
self-destructive. The implementation used by the HP 48 pads the final
packet with null bytes (`0x00`), and Alum trims these from the
received file. However, some files have necessary `0x00` bytes at
their end, and sending these files via XModem causes the object to
become corrupted. One file that suffers from this is the tool
[`FIXIT`](https://www.hpcalc.org/details/2416), by Joe Horn and Mika
Heiskanen. **Conn4x suffers from this same issue, even with this
file. It is a limitation of XModem. If you have sensitive files, or
cannot get checksums to match, send them via Kermit.**

# Contribute
I have documented the XModem server protocol and some HP 48 Kermit
information on the [Alum
wiki](https://github.com/liamhays/alum/wiki). Feel free to edit or add
to the wiki if you have information you'd like to share.

If you have any problems or feature requests for Alum, open an issue
on GitHub or contact me at Liam Hays on the [HP
Forums](hpmuseum.org/forums).

If you'd like to work on Alum, simply install Rust from
[rustup.rs](rustup.rs), then clone the repository and use `cargo` to
build. Note that `cargo` doesn't like if you try to use `cargo run`
with global flags (like `cargo run -f xsend Arkalite.lib`), so you may
have to use `cargo build` and run Alum directly from
`target/debug/alum`.

Finally, if you like Alum and have an HP Meta Kernel calculator---a
48gII, a 49G, a 49g+, or a 50g---and do not need it anymore, please
consider selling it or donating it to me so I can improve
Alum. Message me on the HP Forums if you are interested.
