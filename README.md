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
a similar task. HPex was written in Python 3 and the GUI framework
wxPython, which meant that the end user needed hundreds of megabytes
of shared libraries and executables to run HPex.

Alum is *not* a GUI because a) I can make better software if I don't
need to focus on GUI programming, b) in my experience, the kind of
person who uses an HP calculator is comfortable in the command line,
and c) as mentioned above, the software dependencies for GUI libraries
are big and bloated. Alum is fast, light, and small.

# Features

Alum can:

- Use automatically detected serial ports
- Transfer over Kermit and XModem, both direct and to a server
- Calculate the checksum of any HP 48 object

Alum is not complicated. Alum does just enough and Alum does it right.

# Usage
At the moment, Alum supports sending and receiving files to and from
128-byte `XRECV` and `XSEND`, as well as full XModem server send and
receive support. The XModem server is integrated into the HP 49
series, accessible via `[right-shift][right arrow]`, and is available
for the HP 48 series at
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

## XModem
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


## Kermit
Sending to Kermit is almost identical to sending via XModem. To send
Arkalite like before:

```bash
Sending "Arkalite.lib" via Kermit...
################################################################################################ 32/32 packets (100%)
File info:
  ROM Revision: R, Object CRC: #44ABh, Object length (bytes): 1776.0
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
for HP 49 objects.

Alum also does not currently support receiving files over Kermit. If
there is significant demand for this feature, I will implement it, but
until then I intend to only have Kermit send available.
