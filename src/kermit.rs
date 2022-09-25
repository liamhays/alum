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
use std::fs::File;
use std::io::Write;

use serialport;
use console::style;
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

// TODO: this panics if it is called on an invalid value
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

// TODO: I don't know why this fails sometimes, but I think it has to
// do with how we read the packet (3 bytes then rest of packet).
fn read_packet(port: &mut Box<dyn serialport::SerialPort>) -> Result<KermitPacket, String> {
    // have to sleep, probably because the calculator is slow
    std::thread::sleep(std::time::Duration::from_millis(300));
    // it seems we have to read 3 bytes, then the rest of the packet
    let mut header: [u8; 3] = [0; 3];
    match port.read(header.as_mut_slice()) {
	Ok(_) => {},
	Err(e) => return Err("failed to read header of packet: ".to_owned() + &e.to_string()),
    }
    //println!("header is {:x?}", header);
    if header[0] != SOH {
	return Err("malformed Kermit packet (SOH missing)".to_owned());
    }

    // LEN field
    let len = unchar(header[1]);
    // this would be len - 1, but we want to also read the CR at the end of the packet.
    let mut rest_of_packet = vec![0 as u8; len as usize];

    // could probably reduce this delay slightly
    // this also seems to be needed only for getting files from the calc
    std::thread::sleep(std::time::Duration::from_millis(50));
    match port.read(rest_of_packet.as_mut_slice()) {
	Ok(_) => {},
	Err(e) => return Err("failed to read packet data: ".to_owned() + &e.to_string()),
    }
    //println!("rest of packet is {:x?}", rest_of_packet);
    // subtract 2 to drop 0x0d and check field, to isolate just data
    // portion and assemble KermitPacket struct.
    let data_field = rest_of_packet[1..(len as usize - 2)].to_vec();
    let packet = KermitPacket {
	// TODO: should len be the `len` variable above, that's been uncharred?
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
	return Err("Error: checksum of received data does not match checksum in packet".to_owned());
    }

    //println!("packet is {:x?}", packet);

    return Ok(packet);
}

// This function will exit the entire program on error.
fn send_packet(p: KermitPacket, bar: &ProgressBar, port: &mut Box<dyn serialport::SerialPort>) {
    // still bytes left but the packet is shorter
    //bar.println(format!("p out of loop is {:x?}", p));
    match port.write(&p.to_vec()) {
    	Ok(_) => {},
	Err(e) => {
	    bar.abandon();
	    crate::helpers::error_handler(format!("Error: failed to write data packet: {}", e));
	},
    }
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'Y' as u8 {
		bar.abandon();
		crate::helpers::error_handler(
		    "Error: no ACK for data (\"D\") packet. Try sending again.".to_string());
	    }
	},
	Err(e) => {
	    bar.abandon();
	    crate::helpers::error_handler(format!("Error: bad \"D\" packet response: {}.", e));
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

fn finish_server(port: &mut Box<dyn serialport::SerialPort>) {
    // "I" packet is identical to "S" except for the packet type.

    // seq can and probably should be 0, and Rust lets you do `&mut 0`
    // legally. Funky, for sure.
    let i_packet = make_init_packet(&mut 0, 'I');
    match port.write(&i_packet) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"I\" packet: {}", e)),
    }
    // could wait for ack but probably don't need to.
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    // we are sending a 'G' packet with 'F' in the data field,
    // which tells the server to finish.
    // we use 0 as the seq number even though the I packet was also 0.
    let f_packet = vec![SOH, 0x24, tochar(0), 'G' as u8, 'F' as u8, 0x34, CR]; // hardcoded CRC
    match port.write(&f_packet) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"GF\" packet: {}", e)),
    }
}

// TODO: this is pretty unreliable and doesn't work with x48 at full
// speed. It has to do with the read_packet() function.

// TODO: (more important) need to handle special characters in the filename

// See the top of this file for what this function actually
// does. There are a lot of match statements, but it's how I catch
// serial port and protocol errors.
pub fn send_file(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>, finish: &bool) {
    let mut seq = 0u32;
    
    let file_contents = crate::helpers::get_file_contents(path);
    
    let s_packet = make_init_packet(&mut seq, 'S');
    match port.write(&s_packet) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"S\" packet: {}", e)),
    }
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'Y' as u8 {
		crate::helpers::error_handler("Error: no ACK for \"S\" packet. Try sending again.".to_string());
	    }
	},
	Err(e) => crate::helpers::error_handler(format!("Error: bad \"S\" packet response: {}.", e)),
    }
    
    let f_packet = make_f_packet(&mut seq, path.file_name().unwrap());
    match port.write(&f_packet) {
    	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write \"F\" packet: {}", e)),
    }
    
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'Y' as u8 {
		crate::helpers::error_handler("Error: no ACK for \"F\" packet. Try sending again.".to_string());
	    }
	},
	Err(e) => crate::helpers::error_handler(format!("Error: bad \"F\" packet response: {}", e)),
    }

    let packet_list = make_packet_list(file_contents, &mut seq);
    let bar = crate::helpers::get_progress_bar(packet_list.len() as u64);
    
    for p in packet_list {
	send_packet(p, &bar, port);
	bar.inc(1);
    }
    //bar.println(format!("seq is {seq}"));
    let z_packet = make_generic_packet(&mut seq, 'Z');
    match port.write(&z_packet) {
    	Ok(_) => {},
	Err(e) => {
	    // abondon() leaves the progress bar in place, finish() clears it.
	    bar.abandon();
	    crate::helpers::error_handler(
		format!("Error: failed to write \"Z\" (end-of-file) packet: {}", e));
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
		format!("Error: failed to write \"B\" (end-of-transmission) packet: {}", e));
	},
    }
    bar.finish();

    if *finish {
	finish_server(port);
    }
}


// TODO: indeterminate progress bar or something similar.
pub fn get_file(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>, overwrite: &bool) -> PathBuf {
    let final_path = match overwrite {
	true => path.to_path_buf(),
	false => crate::helpers::get_unique_path(path.to_path_buf()),
    };
    let final_fname = final_path.file_name().unwrap().to_str().unwrap();
    
    let pb = crate::helpers::get_spinner(
	format!("Receiving file as {} from {}...",
		style(final_fname).yellow().bright(),
		style(port.name().unwrap()).green().bright()));

    
    let mut seq = 0;
    let mut out = File::create(&final_path).unwrap();

    // read S packet, which initializes connection from the calculator
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'S' as u8 {
		crate::helpers::error_handler("Error: failed to read \"S\" packet.".to_string());
	    }
	},
	Err(e) => crate::helpers::error_handler(format!("Error: bad \"S\" packet response: {}.", e)),
    }

    std::thread::sleep(std::time::Duration::from_millis(300));
    // ack the S packet with a send-init packet of our own
    let s_ack_packet = make_init_packet(&mut seq, 'Y');
    match port.write(&s_ack_packet) {
    	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(
	    format!("Error: failed to write \"Y\" packet for \"S\" packet: {}", e)),
    }
    
    std::thread::sleep(std::time::Duration::from_millis(300));
    // read F packet, which includes filename
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'F' as u8 {
		crate::helpers::error_handler("Error: failed to read \"F\" packet".to_string());
	    }
	},
	Err(e) => crate::helpers::error_handler(format!("Error: bad \"F\" packet: {}", e)),
    }

    // generic ack the F packet
    let f_ack_packet = make_generic_packet(&mut seq, 'Y');
    match port.write(&f_ack_packet) {
    	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(
	    format!("Error: failed to write \"Y\" packet for \"F\" packet: {}", e)),
    }

    let mut file_bytes: Vec<u8> = Vec::new();
    let mut packet_counter = 0;
    
    loop {
	let packet: KermitPacket = match read_packet(port) {
	    Ok(packet) => {

		if packet.ptype == 'D' as u8 {
		    packet
		} else if packet.ptype == 'Z' as u8 {
		    // Z (end-of-file) is sent by the calc
		    break;
		} else {
		    crate::helpers::error_handler(
			format!("Error: unexpected packet type when waiting for \"D\" packet."));
		    KermitPacket {data: Vec::new(), len: 0, ptype: 0u8, seq: 0}
		}
	    },
	    Err(e) => {
		crate::helpers::error_handler(format!("Error: bad \"D\" packet: {}.", e));
		KermitPacket {data: Vec::new(), len: 0, ptype: 0u8, seq: 0}
	    }
	};

	// convert funky Kermit data format into raw bytes
	let mut i = 0;
	while i < packet.data.len() {
	    let c = *packet.data.get(i).unwrap();
	    if c == '#' as u8 {
		// if the character is a #, then the following char
		// has low 7 bits <= 31 or == 127, or == '#'. The
		// following char is also stored as ctl(c).

		file_bytes.push(ctl(*packet.data.get(i+1).unwrap() as u8));
		i += 2;
	    } else {
		file_bytes.push(*packet.data.get(i).unwrap() as u8);
		i += 1;
	    }
	}

	// send ACK for this packet
	let d_ack_packet = make_generic_packet(&mut seq, 'Y');
	match port.write(&d_ack_packet) {
    	    Ok(_) => {},
	    Err(e) => crate::helpers::error_handler(
		format!("Error: failed to write \"Y\" packet for \"D\" packet: {}", e)),
	}
	packet_counter += 1;
    }

    std::thread::sleep(std::time::Duration::from_millis(300));
    // read Z (EOF) packet from calculator
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'Z' as u8 {
		// TODO: "unexpected packet type" is a great error to throw.
		crate::helpers::error_handler("Error: unexpected packet type after data packets".to_string());
	    }
	},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to read \"Z\" packet: {}", e)),
    }

    let z_ack_packet = make_generic_packet(&mut seq, 'Y');
    match port.write(&z_ack_packet) {
    	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(
	    format!("Error: failed to write \"Y\" packet for \"Z\" packet: {}", e)),
    }

    // read B (EOT) packet from calculator
    match read_packet(port) {
	Ok(packet) => {
	    if packet.ptype != 'B' as u8 {
		crate::helpers::error_handler("Error: unexpected packet type after \"Z\" packet".to_string());
	    }
	},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to read \"B\" packet: {}", e)),
    }

    let b_ack_packet = make_generic_packet(&mut seq, 'Y');
    match port.write(&b_ack_packet) {
    	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(
	    format!("Error: failed to write \"Y\" packet for \"B\" packet: {}", e)),
    }

    

    match out.write_all(&file_bytes) {
	Ok(_) => {},
	Err(e) => panic!("Error: failed to write to output file: {:?}", e),
    };

    pb.finish_with_message(
	format!("Receiving file as {:?} from {}...{} Got {:?} {}.",
		style(final_fname).yellow().bright(),
		style(port.name().unwrap()).green().bright(),
		style("done!").green().bright(),
		packet_counter,
		match packet_counter {
		    1 => "packet",
		    _ => "packets",
		}
	)
    );

    return final_path;
}
