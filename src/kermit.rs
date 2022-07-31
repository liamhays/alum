// This code frequently references the Kermit Protocol Manual, found
// at https://www.kermitproject.org/kproto.pdf.

// Basic file transfer to calculator looks like this:
/*
 * Computer sends "S" packet to calculator
 * Calculator ACKs
 * Computer sends "F" packet to calculator
 * Calculator ACKs
 * Computer sends n "D" (data) packets to calculator and gets ACK for each one
 * Computer sends "Z" (EOF) packet
 * Calculator ACKs
 * Computer sends "B" (EOT) packet
 * Calculator ACKs
 */
// The finish command is done through a server packet.


use std::path::PathBuf;
use std::ffi::OsStr;

use serialport;
use indicatif::ProgressBar;

const SOH: u8 = 0x01;
const CR: u8 = 0x0d;


#[derive(Debug)]
struct KermitPacket {
    len: u8, // packet length - 2
    seq: u8,
    ptype: u8,
    data: Vec<u8>,

    // SOH and CR never charge, so they are in to_vec().
}


impl KermitPacket {
    fn calc_check(&self) -> u8 {
	let v = self.to_vec();
	// oddly, index value LEN is the check value
	v[unchar(self.len) as usize]
    }
    // calculate check and return full packet including EOL.
    fn to_vec(&self) -> Vec<u8> {
	let mut p: Vec<u8> = Vec::new();
	p.push(SOH); // MARK
	p.push(self.len);
	p.push(self.seq);
	p.push(self.ptype);
	for c in &self.data {
	    p.push(*c);
	}
	p.push(block_check_1(p[1..].to_vec()));
	p.push(CR); // packet EOL
	return p;
    }
}


// tochar(), unchar(), and ctl() are implemented as described on page
// 5 of the protocol manual.
fn tochar(c: u8) -> u8 {
    c + 32
}

fn unchar(c: u8) -> u8 {
    c - 32
}

fn ctl(c: u8) -> u8 {
    c ^ 64
}

fn block_check_1(data: Vec<u8>) -> u8 {
    // Calculate Kermit block check type 1 on data.
    // map to u32 to prevent overflow
    let s: u32 = data.iter().map(|&b| b as u32).sum();
    return tochar((s + ((s & 192) / 64) & 63) as u8);
}

// Make an S (or any packet type specified in ptype) packet and increment `seq`.

// We are emulating a very basic Kermit: only type 1 block check and a
// couple commands.
fn make_init_packet(seq: &mut u32, ptype: char) -> Vec<u8> {
    // "S" packet is Send-Init, and establishes connection schema.
    
    // The LEN field must be correct, or the calculator will do
    // exactly nothing when we send a packet.
    let packet_data: Vec<u8> = vec![
	// MAXL     TIME       NPAD       PADC    EOL         QCTL       QBIN       CHKT
	tochar(94), tochar(2), tochar(0), ctl(0), tochar(CR), '#' as u8, 'Y' as u8, '1' as u8];

    // extra info on these fields.
    // PADC is ctl(0) because NPAD (number of padding chars) is also zero.
    // EOL is CR, the default
    //   - the HP 48 sends QCTL, QBIN, and CHKT, so we do too.
    // QCTL: '#' is default
    // QBIN: ASCII char used to quote for 8th bit set, we use 'Y' to
    // say "I agree to what you want but don't need 8-bit quoting".
    // CHKT: check type, we only support type 1.

    let s_packet = KermitPacket {
	len: tochar(11),
	seq: tochar((*seq as u8) % 64),
	ptype: ptype as u8,
	data: packet_data,
    };
    
    *seq += 1;
    
    return s_packet.to_vec();
}

// Make an F packet with the data portion the contents of `fname`, set
// the length field, and increment `seq`.
fn make_f_packet(seq: &mut u32, fname: &OsStr) -> Vec<u8> {
    // "F" packet is File-Header and contains filename.
    let mut packet_data: Vec<u8> = Vec::new();

    for c in fname.to_str().unwrap().chars() {
	packet_data.push(c as u8);
    }

    let f_packet = KermitPacket {
	// 2 because seq and 'F', 1 because block check char
	len: tochar((fname.len() + 2 + 1) as u8),
	seq: tochar((*seq as u8) % 64),
	ptype: 'F' as u8,
	data: packet_data,
    };

    
    *seq += 1;
    
    return f_packet.to_vec();
}

// Make a packet of type `ptype` and no data portion. Increment `seq`.
fn make_generic_packet(seq: &mut u32, ptype: char) -> Vec<u8> {
    let p = KermitPacket {
	len: tochar(3u8),
	seq: tochar((*seq as u8) % 64),
	ptype: ptype as u8,
	// no data, just insert empty vector
	data: Vec::new(),
    };
    *seq += 1;
    return p.to_vec();
}

// eventual todo: should use Result instead of Option
fn read_packet(port: &mut Box<dyn serialport::SerialPort>) -> Option<KermitPacket> {
    // have to sleep, probably because the calculator is slow
    std::thread::sleep(std::time::Duration::from_millis(300));
    // it seems we have to read 3 bytes, then the rest of the packet
    let mut header: [u8; 3] = [0; 3];
    match port.read(header.as_mut_slice()) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to read header of packet: {:?}", e)),
    }
    
    if header[0] != SOH {
	eprintln!("SOH missing from packet.");
	// something is very wrong
	return None;
    }

    // LEN field
    let len = unchar(header[1]);
    // this would be len - 1, but we want to also read the CR at the end of the packet.
    let mut rest_of_packet = vec![0 as u8; len as usize];

    match port.read(rest_of_packet.as_mut_slice()) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to read packet data: {:?}", e)),
    }
    
    // subtract 2 to drop 0x0d and check field, to isolate just data
    // portion and assemble KermitPacket struct.
    let data_field = rest_of_packet[1..(len as usize - 2)].to_vec();
    let packet = KermitPacket {
	len: header[1],
	seq: header[2],
	ptype: rest_of_packet[0],
	// clone to create non-local object, otherwise rx_data goes
	// out of scope at the end of this function and refuses to
	// compile
	data: data_field.clone(),
    };
    
    let rx_checksum = rest_of_packet[len as usize - 3];
    // verify checksum on packet
    if rx_checksum != packet.calc_check() {
	return None;
    }

    return Some(packet);
}

// This function will exit the entire program on error.
fn send_packet(p: KermitPacket, bar: &ProgressBar, port: &mut Box<dyn serialport::SerialPort>) {
    // still bytes left but the packet is shorter
    //bar.println(format!("p out of loop is {:x?}", p));
    match port.write(&p.to_vec()) {
    	Ok(_) => {},
	Err(e) => {
	    bar.abandon();
	    crate::helpers::error_handler(format!("Error: failed to write final data packet: {:?}", e));
	},
    }
    let response = read_packet(port);
    match response {
	None => {
	    bar.abandon();
	    crate::helpers::error_handler(
		"Error: got no or invalid response for final data (\"D\") packet. Try sending again.".to_string());
	}
	_ => {
	    if response.unwrap().ptype != 'Y' as u8 {
		bar.abandon();
		crate::helpers::error_handler(
		    "Error: no ACK for final data (\"D\") packet. Try sending again.".to_string());
	    }
	},
    }
}

// Make a Vec of KermitPackets from the contents of the file, specified in `f`.
fn make_packet_list(f: Vec<u8>, seq: &mut u32) -> Vec<KermitPacket> {
    let mut packet_list: Vec<KermitPacket> = Vec::new();
    let mut packet_data: Vec<u8> = Vec::new();
    let mut bytes_added = 0u32;
    
    for c in f {
	// Kermit specification says that any byte whose low 7 bits
	// form a control character must be changed to the control
	// prefix char (in this case '#') followed by ctl(byte).
	let low_7bits = c & 0x7f;
	if low_7bits <= 31 || low_7bits == 127 {
	    packet_data.push('#' as u8);
	    packet_data.push(ctl(c));
	    bytes_added += 2;
	} else if low_7bits == '#' as u8 {
	    // It might seem that we would want to check if c is '#',
	    // but the manual specifically says to consider only the 7
	    // low bits to check if a character is the prefix
	    // character. However, we still have to push all 8 bits
	    // onto the packet afterward.
	    packet_data.push('#' as u8);
	    packet_data.push(c);
	    bytes_added += 2;
	} else {
	    packet_data.push(c);
	    bytes_added += 1;
	}

	// The whole control prefix issue means that the packet length
	// can change. 84 is the minimum number of bytes in the data
	// field that our packets will have.
	if bytes_added > 84 {
	    packet_list.push(KermitPacket {
		len: tochar(bytes_added as u8 + 3),
		seq: tochar((*seq as u8) % 64),
		ptype: 'D' as u8,
		data: packet_data,
	    });

	    *seq += 1;
	    bytes_added = 0;
	    packet_data = Vec::new();
	}
    }
    //bar.println(format!("bytes_added is {:x?}", bytes_added));
    if bytes_added != 0 {
	packet_list.push(KermitPacket {
	    len: tochar(bytes_added as u8 + 3),
	    seq: tochar((*seq as u8) % 64),
	    ptype: 'D' as u8,
	    data: packet_data,
	});
	*seq += 1;
    }
    return packet_list;
}

// TODO: finish server command

// TODO: this doesn't work with x48 at full speed

// See the top of this file for what this function actually
// does. There are a lot of match statements, but it's how I catch
// serial port and protocol errors.
pub fn send_file(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>, finish: bool) {
    let mut seq = 0u32;
    
    let file_contents = crate::helpers::get_file_contents(path);
    
    let s_packet = make_init_packet(&mut seq, 'S');
    match port.write(&s_packet) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"S\" packet: {:?}", e)),
    }
    let mut response = read_packet(port);
    match response {
	None => crate::helpers::error_handler("Error: got no or invalid response for \"S\" packet.".to_string()),
	_ => {
	    if response.unwrap().ptype != 'Y' as u8 {
		crate::helpers::error_handler("Error: no ACK for \"S\" packet. Try sending again.".to_string());
	    }
	},
    }
    
    let f_packet = make_f_packet(&mut seq, path.file_name().unwrap());
    match port.write(&f_packet) {
    	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"F\" packet: {:?}", e)),
    }
    response = read_packet(port);
    match response {
	None => crate::helpers::error_handler("Error: got no or invalid response for \"F\" packet.".to_string()),

	_ => {
	    if response.unwrap().ptype != 'Y' as u8 {
		crate::helpers::error_handler("Error: no ACK for \"F\" packet. Try sending again.".to_string());
	    }
	},
    }

    let packet_list = make_packet_list(file_contents, &mut seq);
    let bar = crate::helpers::get_progress_bar(packet_list.len() as u64);
    
    for p in packet_list {
	send_packet(p, &bar, port);
	bar.inc(1);
    }
    bar.println(format!("seq is {seq}"));
    let z_packet = make_generic_packet(&mut seq, 'Z');
    match port.write(&z_packet) {
    	Ok(_) => {},
	Err(e) => {
	    // abondon() leaves the progress bar in place, finish() clears it.
	    bar.abandon();
	    crate::helpers::error_handler(
		format!("Error: failed to write \"Z\" (end-of-file) packet: {:?}", e));
	},
    }

    // needed to make sure the calculator gets its packets
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    let b_packet = make_generic_packet(&mut seq, 'B');
    match port.write(&b_packet) {
    	Ok(_) => {},
	Err(e) => {
	    bar.abandon();
	    crate::helpers::error_handler(
		format!("Error: failed to write \"B\" (end-of-transmission) packet: {:?}", e));
	},
    }
    bar.finish();

    if finish {
	// "I" packet is identical to "S" except for the packet type.
	let i_packet = make_init_packet(&mut seq, 'I');
	match port.write(&i_packet) {
	    Ok(_) => {},
	    Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"I\" packet: {:?}", e)),
	}
	// could wait for ack but probably don't need to.
	std::thread::sleep(std::time::Duration::from_millis(300));
	// note: sending the I packet resets the seq number.
	
	// we are sending a 'G' packet with 'F' in the data field,
	// which tells the server to finish.
	let f_packet = vec![SOH, 0x24, tochar(0), 'G' as u8, 'F' as u8, 0x34, CR]; // hardcoded CRC
	match port.write(&f_packet) {
	    Ok(_) => {},
	    Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"GF\" packet: {:?}", e)),
	}
	
    }
}
