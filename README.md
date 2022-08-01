# alum
Alum is a file transfer tool for HP's RPL calculators.

# Why Alum?
Alum (pronounced like the first part of "aluminum") is a command-line
application for transferring files to and from your calculator.

Alum is intended as a replacement for software like [the Connectivity
Kit](https://www.hpcalc.org/details/5890). Alum is better because:

- It is cross-platform, currently tested on Windows and Linux
- It is programmed in Rust, meaning it requires no external
  dependencies (except for one small package on Linux)
- Alum uses homegrown implementations of XModem and Kermit, which
  makes Alum maintainable long-term and also reduces the external
  dependency count.
  
I am retiring my previous software HPex, a GUI tool that accomplished
a similar task. HPex was written in Python 3 and wxPython, which meant
that the end user needed hundreds of megabytes of shared libraries and
executables to run HPex.

Alum is *not* a GUI because a) I can make better software if I don't
need to focus on GUI programming, b) in my experience, the kind of
person who uses an HP calculator is comfortable in the command line,
and c) as mentioned above, the software dependencies for GUI libraries
are big and bloated. Alum is fast, light, and small.

# Features

Alum can:

- Automatically detect a serial port to use
- Transfer via Kermit and XModem, both direct and to a server
- Calculate the checksum and size of any HP 48 object

Alum is not complicated. Alum does just enough and Alum does it right.

# Usage
Download a binary from the Releases page and place it somewhere
convenient. You may wish to add it to your `PATH` variable.

For full usage information, run Alum with no arguments.

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

```bash
$ alum xsend Arkalite.lib
Sending "Arkalite.lib" to XModem server...
################################################################################################  7/7  packets (100%)
Done!
File info:
  ROM Revision: X, Object CRC: #44ABh, Object length (bytes): 1776.0
```

In this example, Alum found the one physical serial port on the system
and used it automatically.


## Kermit transfers
Sending to Kermit is almost identical to sending via XModem. To send
Arkalite like before:

```bash
Sending "Arkalite.lib" via Kermit...
################################################################################################ 32/32 packets (100%)
File info:
  ROM Revision: R, Object CRC: \#44ABh, Object length (bytes): 1776.0
```

## Extra transfer features
To finish or close any server after a transfer, pass the `-f` flag to
Alum, like this: `alum -f ksend Arkalite.lib`. If the file transfer is
successful, Alum will send a quit command to the calculator after the
transfer.

# Limitations
Alum has not been tested with any HP 49 series calculator. In fact,
Alum will not even calculate the checksum of an HP 49 object, because
I have found that the algorithm I use for HP 48 objects doesn't work
for HP 49 objects. Alum also does not support the 1K CRC direct
`XRECV` and `XSEND` added in the HP 49.

Alum also does not currently support receiving files over Kermit. If
there is significant demand for this feature, I will implement it, but
until then I intend to only have Kermit send available.

## Future features
- [ ] XModem server and Kermit server file listing
- [ ] 1K CRC direct XModem
- [ ] HP 49 object info
- [ ] (possibly) Kermit receive

# Contribute
If you'd like to work on Alum, simply install Rust from
[rustup.rs](rustup.rs). Then clone the repository and use `cargo` to
build. Note that `cargo` doesn't like if you try to use `cargo run`
with global flags (like `cargo run -f xsend Arkalite.lib`), so you may
have to use `cargo build` and run Alum directly from
`target/debug/alum`.

Furthermore, if you would like to see features implemented in Alum,
feel free to message me at Liam Hays on the MoHPC forums or start a
new thread. If you like Alum and have an HP Meta Kernel calculator---a
48gII, a 49G, a 49g+, or a 50g---and do not need it anymore, please
consider selling it or donating it to me so I can improve
Alum. Message me on the HP Forums if you are interested.
