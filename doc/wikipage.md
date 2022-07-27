I extracted this information from the [Conn4x Pascal source
code](https://www.hpcalc.org/details/5416) as well as analyzing USB
traffic from the [HP Connectivity
Kit](https://www.hpcalc.org/details/5890) with Wireshark. Code
examples are in Python because Python is prevalent, but could be
adapted to any language.

For information about XModem, I recommend looking at
[http://wiki.synchro.net/ref:xmodem](http://wiki.synchro.net/ref:xmodem). This
is an in-depth archive of XModem information (it's both a useful
reference and a fun look into the past: "Perhaps when 50% of the
households outgrow the Commodore 64...we will see the...end for
XModem").

This document assumes the reader has knowledge of the normal XModem
protocol.

# Basic XModem Server Protocol
## Commands
A command is issued to the XModem server by sending a single raw byte
over serial.

## Command Packets
A command packet is a sequence of bytes containing a header, the
packet contents, and a checksum. Command packets are sent by both the
calculator and the computer and are always in the same format.

For example, suppose we want to make a packet containing the text
`IOPAR`. Start by making the header, which is two bytes, representing
a big-endian size of the text in the packet (i.e., the first byte of
the header is the higher-order byte of the number). For `IOPAR`, the
header would be the bytes `\x00\x05`, because `IOPAR` is five bytes
long.

Now, define a function `checksum`.

```python
def checksum(s: str) -> int:
    result = 0
    for i in s:
        result += ord(i)
    return result
```

The checksum simply adds the ASCII value of every character in the
string together. In the final packet, the checksum is only one byte,
which is the lowest byte of the checksum (`& 0xff`)

Finally, to assemble the command packet, append the header, packet
data, and checksum together in that order. For a packet containing
`IOPAR`, the final packet will be `\x00\x05IOPAR\x7b`.

## XModem transfers
All versions of the XModem server can receive 128-byte and 1Kbyte
XModem transfers, but only send 128-byte transfers.

The server uses a slightly modified XModem. For all packet types, the
checksum is two bytes at the end of the packet, calculated as
follows. I call this the Conn4x CRC algorithm.

Define a function `init_crc_array()` that creates a list:

```python
def init_crc_array():
	crc_array = []
	for crc in range(16):
		for inp in range(16):
			crc_array.append((crc ^ inp) * 0x1081)
```

To actually calculate the checksum on a string of bytes, use a
function like this:

```python
def crc_bytes(s: bytes) -> int:
	crc_arr = init_crc_array()
	result = 0
	for i in s:
		k = (result & 0xf) << 4
		result = (result >> 4) ^ crc_arr[k + (i & 0xf)]
		k = (result & 0xf) << 4
		result = (result >> 4) ^ crc_arr[k + (i >> 4)]
		
	return result
```

Both of these functions are converted from the Conn4x source code.

To assemble a packet, create a normal packet header (either `SOH seq
inv_seq` or `STX seq inv_seq` depending on packet size), append the
data, then append the two lowest bytes of the Conn4x CRC value. This
is done in the same big-endian order as the header of a command packet
(see above).

I theorize that this CRC algorithm is identical to the calculator's
internal CRC algorithm, but I have not tested it.

## Running Transfers
### Sending to the calculator
The calculator sends the character `D`, which tells the computer that
the calculator wants an XModem transfer using the Conn4x CRC
algorithm. This is equivalent a normal XModem receiver sending ASCII
`NAK`.

Once the computer receives `D`, it begins sending packets and waiting
for ACK, just like normal XModem.

The Conn4x CRC algorithm is used on both 1K and 128-byte packets.

The calculator only sends `D` a total of 4 times before giving up and
accepting the XModem transfer. This is unreliable, though, and
shouldn't be used. The calculator also waits many seconds between
individual `D` sends.

### Receiving from the calculator
Receiving a file from the XModem server uses only 128-byte XModem. It
is identical to a normal XModem transfer (no `D` like in sending),
EXCEPT that the calculator uses the Conn4x CRC algorithm, creating
packets as described above.

### Cancelling a send
Conn4x sends 3 `CAN` characters at once to cancel a send to the
calculator. This seems to only work if a 1K packet is being sent---I
think it's ignored on 128-byte packets.

# Command Library
## Get free memory (`M`)
Send this command, wait 300 milliseconds, and read a command
packet. The packet's text component is ASCII text of the number of
free bytes on the calculator.
## Execute (`E`)
Send this command, then send a command packet with the User RPL
command(s) you want to execute on the calculator. There is no way to
get a response back, except for storing data in a variable and
transferring that variable.
## Kill server (`Q`)
This command ends the server on the calculator.
## Put file (`P`)
To upload a file to the calculator, send this command and then send a
command packet with the destination filename. Then, you can start an
XModem transfer to the calculator.
## Get file (`G`)
To get a file from the calculator, send this command and then send a
command packet with the desired object name. Start an XModem transfer
to receive the file.
## Server version (`V`)
Send this command, then read a command packet. The packet will contain
a string, something like "XModem Version 1.010".
## Directory listing (`L`)
Send this command, wait 300 milliseconds, and then read a command
packet. This command packet consists of groups of data about each
variable in the current directory (I think the server defaults to
`HOME`). Each group consists of the following:
	
| 1 byte  | object name length |
| n bytes | object name        |
| 2 bytes | object prolog      |
| 3 bytes | object size        |
| 2 bytes | object CRC         |

All multi-byte values are little-endian (lowest byte comes
first). Each group follows one another in a long string.
## Unknown (`o`)
This command is in the Conn4x source and is called "ExecFile", but I
don't know what it does (it may not even be implemented in the
server). It is followed by a command packet.

# Conn4x Oddities
## Initial connect
Because the XModem server doesn't implement any way to read data off
the stack, Conn4x runs a RPL command and stores the result to a
file. The command is: `-36 FS? -51 FS? 2 \->LIST \->STR VERSION DROP +
'$$$t' STO`. Flag -36 is the overwrite flag, and -51 is the fraction
mark. Conn4x uses these for something, I don't know what, and uses the
ROM version as diagnostic output in the Conn4x window.

## Text mode
Conn4x supports getting files in text mode even though the XModem
server doesn't. To do this, it relies on the calculator to convert to
text, with this command: `RCLF 64 STWS STD -17 FS? -18 FS? -51 FS? IOPAR 6
GET 4 \->LIST \->STR 'PRTPAR' RCL \->STR + '$$$t' STO STOF`

The string above uses `'PRTPAR'` as an example variable. This command
assembles a string with the angle mode, fraction mark, and the
translate mode (`IOPAR 6 GET`). The variable is converted to a string,
and the four elements on the stack are all placed in a list, which is
stored in `'$$$t'`.

Conn4x parses this file and extracts the text element.

## Other checksums
The Conn4x source contains code for other types of checksums. It might
just be remnants from previous development.
 
