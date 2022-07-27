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

const SOH: u8 = 0x01;
const CR: u8 = 0x0d;

// We should probably use some kind of struct for each packet and
// implement a to_vec() function, or something like that.

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

fn make_s_packet(seq: &mut u32) -> Vec<u8> {
    // "S" packet is Send-Init, and establishes connection schema.
    
    // The LEN field must be correct, or the calculator will do
    // exactly nothing when we send a packet.
    let mut s_packet: Vec<u8> = Vec::new();
    // fields labeled as in Kermit docs
    s_packet.push(SOH); // MARK
    s_packet.push(tochar(11)); // LEN, aka packet length - 2
    // sequence number loops at 64
    s_packet.push(tochar((*seq as u8) % 64)); // SEQ
    s_packet.push('S' as u8); // TYPE
    // append DATA portion, this is made up of special fields for the "S" packet.
    s_packet.push(tochar(94)); // MAXL (max packet length), this is default
    s_packet.push(tochar(2)); // TIME (timeout), this is rounded up from serial port timeout
    s_packet.push(tochar(0)); // NPAD (number of padding chars), not needed here
    s_packet.push(ctl(0)); // PADC (padding char), N/A because NPAD = 0
    s_packet.push(tochar(CR)); // EOL (end of packet char), CR is the default
    // The following two fields are optional, but the HP 48 sends them.
    s_packet.push('#' as u8); // QCTL (quote control char), '#' is the default
    // QBIN (ASCII char used to quote for 8th bit set), Y means I
    // agree but don't need it. HP 48 sends ' ', meaning no 8-bit
    // quoting
    s_packet.push('Y' as u8);
    s_packet.push('1' as u8); // CHKT (check type), we only support type 1
    s_packet.push(block_check_1(s_packet[1..].to_vec()));
    s_packet.push(CR);

    *seq += 1;
    
    return s_packet;
}

fn make_f_packet(seq: &mut u32, fname: &OsStr) -> Vec<u8> {
    // "F" packet is File-Header and contains filename.
    let mut f_packet: Vec<u8> = Vec::new();
    f_packet.push(SOH);
    f_packet.push(tochar(0)); // set later
    f_packet.push(tochar((*seq as u8) % 64));
    f_packet.push('F' as u8);

    for c in fname.to_str().unwrap().chars() {
	f_packet.push(c as u8);
    }
    // 2 because seq and 'F', 1 because block check char
    f_packet[1] = tochar((fname.len() + 2 + 1) as u8);
    f_packet.push(block_check_1(f_packet[1..].to_vec()));
    f_packet.push(CR);

    *seq += 1;
    
    return f_packet;
}

fn make_generic_packet(seq: &mut u32, ptype: char) -> Vec<u8> {
    let mut p: Vec<u8> = Vec::new();
    p.push(SOH);
    p.push(tochar(3u8));
    p.push(tochar((*seq as u8) % 64));
    p.push(ptype as u8);
    p.push(block_check_1(p[1..].to_vec()));
    p.push(CR);
    *seq += 1;
    println!("{:x?}", p);
    return p;
}

fn start_d_packet(seq: u32) -> Vec<u8> {
    let mut d_packet: Vec<u8> = Vec::new();
    d_packet.push(SOH);
    // placeholder value
    d_packet.push(tochar(0u8));
    d_packet.push(tochar((seq as u8) % 64));
    d_packet.push('D' as u8);

    return d_packet;
}

fn read_packet(port: &mut Box<dyn serialport::SerialPort>) -> Option<Vec<u8>> {
    // have to sleep---maybe the buffer has to accumulate or something?
    std::thread::sleep(std::time::Duration::from_millis(300));
    // it seems we have to read 3 bytes, then the rest of the packet
    let mut header: [u8; 3] = [0; 3];
    match port.read(header.as_mut_slice()) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to read header of packet: {:?}", e)),
    }
    //println!("header: {:x?}", header);
    
    if header[0] != SOH {
	// something has gone seriously wrong, because the packet doesn't start with SOH
	return None;
    }

    let len = unchar(header[1]);
    // this would be len - 1, but we want to also read the CR at the end.
    let mut packet_data = vec![0 as u8; len as usize];
    match port.read(packet_data.as_mut_slice()) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to read packet data: {:?}", e)),
    }
    //println!("packet_data: {:x?}", packet_data);

    let mut full_packet: Vec<u8> = Vec::new();
    // we need to append the header and the rest of the data, and I
    // think this is the best way to do so.
    full_packet.push(header[0]);
    full_packet.push(header[1]);
    full_packet.push(header[2]);
    full_packet.append(&mut packet_data);
    
    //println!("full_packet: {:x?}", full_packet);
    // add 1 to length because LEN field does not include the LEN field itself.
    let check_part = full_packet[1..(len + 1) as usize].to_vec();
    let checksum = block_check_1(check_part);
    // verify checksum on packet
    if checksum != full_packet[len as usize + 1] {
	return None;
    }
    //println!("checksum matches");
    return Some(full_packet);
}




    
pub fn send_file(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>) {
    let file_contents = crate::helpers::get_file_contents(path);

    // We are emulating a very basic Kermit: only type 1 block check
    // and a couple commands.
    let mut seq = 0u32;

    let s_packet = make_s_packet(&mut seq);
    
    println!("s_packet: {:x?}", s_packet);

    port.write(&s_packet);
    let mut response = read_packet(port);
    if response == None {
	crate::helpers::error_handler("Error: got invalid or no response for \"S\" packet, exiting".to_string());
    }
    println!("response: {:x?}", response);

    let f_packet = make_f_packet(&mut seq, path.file_name().unwrap());

    println!("f_packet: {:x?}", f_packet);
    port.write(&f_packet);
    response = read_packet(port);
    if response == None || response.as_ref().unwrap()[3] != 'Y' as u8 {
	crate::helpers::error_handler("Error: got invalid or no response for \"F\" packet, exiting".to_string());
    }
    println!("response: {:x?}", response);

    //let packet_count = crate::helpers::div_up(file_contents.len(), 86); // see make_d_packet about 86.

    let mut d_packet: Vec<u8> = start_d_packet(seq);
    let mut bytes_added = 0u32;
    for c in file_contents {
	let low_7bits = c & 0x7f;
	if low_7bits <= 31 || low_7bits == 127 {
	    d_packet.push('#' as u8);
	    d_packet.push(ctl(c));
	    bytes_added += 2;
	    //println!("adding #");
	} else {
	    d_packet.push(c);
	    bytes_added += 1;
	}
	//println!("before check, bytes_added is {:?}", bytes_added);
	if bytes_added > 83 {
	    println!("bytes_added is {:?}", bytes_added);
	    d_packet[1] = tochar(bytes_added as u8 + 3);
	    d_packet.push(block_check_1(d_packet[1..].to_vec()));
	    d_packet.push(CR);
	    println!("d_packet sent: {:x?}, len is {:?}, seq is {:?}", d_packet, d_packet.len(), seq);
	    port.write(&d_packet);
	    seq += 1;
	    println!("response: {:x?}", read_packet(port));
	    bytes_added = 0;
	    
	    d_packet = start_d_packet(seq);
	}
    }
    if bytes_added != 0 {
	// still bytes left but the packet is shorter
	d_packet[1] = tochar((bytes_added + 3) as u8);
	d_packet.push(block_check_1(d_packet[1..].to_vec()));
	d_packet.push(CR);
	println!("d_packet out of loop is {:x?}", d_packet);
	port.write(&d_packet);
	seq += 1;
	println!("response: {:x?}", read_packet(port));
    }
    
    

    let z_packet = make_generic_packet(&mut seq, 'Z');
    port.write(&z_packet);
    println!("response: {:x?}", read_packet(port));
    
    let b_packet = make_generic_packet(&mut seq, 'B');
    port.write(&b_packet);
    println!("response: {:x?}", read_packet(port));
}
