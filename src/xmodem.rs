// This module needs to take a file path, read its contents into
// memory, and send out packets. I don't know how the packet sending
// should be done at this point.

// It also needs to work with 1024- and 128-byte packets, for the
// XModem server.





use std::path::PathBuf;
use std::path::Path;
use std::ffi::OsStr;
use std::fs::File;
use std::thread;
use std::time::Duration;
use std::io::Write;

use serialport;

#[derive(PartialEq)]
enum ChecksumMode {
    Normal,
    Conn4x,
}

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a; // used as packet filler, ascii code SUB (substitute)

// packet_count_offset is an adjustment to the packet-counting loop
// inside this function. For example, if the offset is 3, then the
// first packet generated by this loop will have packet number 4
// (XModem uses 1-indexed packet numbers). As such, 1 may need to be
// added to the offset to get the desired outcome.
fn data_to_128_packets(data: &Vec<u8>, packet_count_offset: usize, checksum_mode: ChecksumMode) -> Vec<Vec<u8>> {
    let mut packet_list = Vec::new();
    // generated even when in normal checksum mode, because it's fast
    // and might be used in the loop.
    let crc_array: [u32; 256] = init_crc_array();

    // If we always add 1 (in the calling function), we'll end up with
    // a scenario where the first packet will have a sequence number
    // of 2 instead of 1. This is mostly an issue for an object that's
    // less than 1024 bytes.

    // Round up packet count because any remaining data still has to
    // fit in a packet.
    let packet_count = crate::helpers::div_up(data.len(), 128);

    // Begin assembling packets
    for i in 0..packet_count {
	// One XModem 128-byte packet contains 128 bytes of data, plus
	// 4 metadata bytes.
	let mut packet = Vec::new();
	let mut checksum = 0u32;
	
	packet.push(SOH);
	// sequence number, when encoded in packet uses 1-indexing.
	// doing the addition this way should prevent overflow errors.
	let seq = (i + 1 + packet_count_offset) as u8;
	packet.push(seq);
	// 1's complement of the sequence number
	packet.push(255u8 - (255u8 & seq));

	// If we're on the last packet (packet_count - 1), go to the end of the file.
	// Otherwise, we'll end up reading past the end of the file and get an error.
	let loop_limit = {
	    if i == packet_count - 1 {
		data.len() % 128
	    } else {
		128
	    }
	};

	
	for j in 0..loop_limit {
	    let byte = *data.get(i * 128 + j).expect("File unexpectedly cut short!");
	    if checksum_mode == ChecksumMode::Normal {
		checksum += byte as u32;
	    }
	    packet.push(byte);
	}

	// fill rest of packet if necessary
	if loop_limit != 128 {
	    for _ in loop_limit..128 {
		if checksum_mode == ChecksumMode::Normal {
		    checksum += SUB as u32;
		}
		packet.push(SUB);
	    }
	}

	// get lowest byte of checksum
	if checksum_mode == ChecksumMode::Conn4x {
	    // calculate Conn4x checksum and get two lowest bytes
	    let crc = crc_conn4x(crc_array, packet[3..].to_vec());
	    packet.push(((crc & 0xff00u32) >> 8) as u8);
	    packet.push((crc & 0xffu32) as u8);
	} else {
	    // get lowest byte of checksum
	    packet.push((checksum & 0xffu32) as u8);
	}
	packet_list.push(packet);
    }

    return packet_list;
}

// Converted straight from Conn4x source. This is not standard with
// the CCITT CRC calculation.
fn init_crc_array() -> [u32; 256] {
    let mut crc_array: [u32; 256] = [0; 256];
    let mut i = 0usize;
    for crc in 0..16 {
	for inp in 0..16 {
	    crc_array[i] = (crc ^ inp) * 0x1081;
	    i += 1;
	}
    }
    return crc_array;
}

fn crc_conn4x(crc_array: [u32; 256], data: Vec<u8>) -> u32 {
    let mut result = 0u32;
    for e in data.iter() {
	let mut k = (result & 0xf) << 4;
	result = (result >> 4) ^ crc_array[(k as u32 + (*e as u32 & 0xfu32)) as usize];
	k = (result & 0xf) << 4;
	result = (result >> 4) ^ crc_array[(k as u32 + (*e as u32 >> 4u32)) as usize];
    }
    
    return result;
}


// Generate a list of XModem packets with Conn4x checksums. This will
// try to make 1K-byte packets, then make 128-byte packets with the
// remaining data.
fn data_to_conn4x_packets(data: &Vec<u8>) -> Vec<Vec<u8>> {
    let mut packet_list = Vec::new();
    let crc_array: [u32; 256] = init_crc_array();
    // Number of 1K-byte packets to use
    let mut packet_offset = 0usize;
    let packet_count = data.len() / 1024;
    //println!("data.len() is {:?}, packet_count in conn4x_packets is {:?}", data.len(), packet_count);
    for i in 0..packet_count {
	packet_offset += 1;
	
	let mut packet = Vec::new();

	packet.push(STX); // STX used to indicate 1K-byte block
	let packet_count = (i + 1) as u8;
	packet.push(packet_count);
	packet.push(255u8 - (255u8 & packet_count as u8));

	// We will always push 1024 in this loop, smaller amounts are
	// relegated to data_to_128_packets
	for j in 0..1024 {
	    let byte = *data.get(i * 1024 + j).expect("File unexpectedly cut short!");
	    packet.push(byte);
	}

	// run CRC on the data portion of the packet
	let crc = crc_conn4x(crc_array, (&packet[3..]).to_vec());
	packet.push(((crc & 0xff00u32) >> 8) as u8);
	packet.push((crc & 0xffu32) as u8);

	packet_list.push(packet);
    }

    // get what remains of the data, this is just everything after the
    // last 1K packet.
    let mut substr_128 = Vec::new();
    for i in &data[packet_count * 1024..] {
	substr_128.push(*i);
    }

    let mut packets_128 = data_to_128_packets(&substr_128, packet_offset, ChecksumMode::Conn4x);
    // Append both vectors together for the final list
    packet_list.append(&mut packets_128);
    
    return packet_list;
}

// Looks for ack_char on `port`, returns true if char found or false if NAK found.
fn wait_for_char(port: &mut Box<dyn serialport::SerialPort>, ack_char: u8) -> u8 {
    let mut buf: [u8; 1] = [0; 1];
    loop {
	match port.read(buf.as_mut_slice()) {
	    Ok(_) => {
		let byte = *buf.get(0).unwrap();
		if byte == ack_char {
		    break;
		} else {
		    return byte;
		}
	    },
	    Err(e) => crate::helpers::error_handler(format!("Error: failed to read char: {:?}", e)),
	}

	//println!("waiting for ACK");
    }
    return ack_char;
}



// The way packets are sent and responses are handled don't change.
fn send_packets(packet_list: &Vec<Vec<u8>>, port: &mut Box<dyn serialport::SerialPort>) {
    let pb = crate::helpers::get_progress_bar(packet_list.len() as u64, "packets".to_string());
    
    for (pos, packet) in packet_list.iter().enumerate() {
	let mut retry_count = 0;
	match port.write(packet) {
	    Ok(_) => {},
	    Err(e) => crate::helpers::error_handler(format!("Error: failed to write packet {:?}", e)),
	}
	
	// wait for ACK on current packet
	let c = wait_for_char(port, ACK);
	if c == NAK {
	    match port.write(packet) {
		Ok(_) => {},
		Err(e) => crate::helpers::error_handler(format!("Error: failed to read char for packet {:?}: {:?}", pos, e)),
	    }
	    retry_count += 1;
	    if retry_count == 3 {
		// Something deeper is wrong, give up
		crate::helpers::error_handler(format!("Error: failed on packet {:?} after 3 tries, giving up.", pos));
	    }
	} else if c == CAN {
	    // cancel, just exit.
	    pb.abandon();
	    crate::helpers::error_handler("Error: transfer cancelled by calculator.".to_string());
	}
	
	pb.inc(1);
	// if we successfully sent the last packet, send EOT after the
	// last ACK. This will trigger another ACK, which we look for
	// below (but probably don't need to)
	if pos == packet_list.len() - 1 && c == ACK {
	    //println!("sending EOT");
	    let wr_buf: [u8; 1] = [EOT];
	    match port.write(&wr_buf) {
		Ok(_) => {},
		Err(e) => crate::helpers::error_handler(format!("Error: failed to send EOT: {:?}", e)),
	    }
	    wait_for_char(port, ACK);

	}
    }
    // make the progress bar visible on screen
    pb.finish();
}



fn finish_server(port: &mut Box<dyn serialport::SerialPort>) {
    // needed to make Q actually work
    thread::sleep(Duration::from_millis(300));
    // send Q to server, which tells server to exit
    let buf: [u8; 1] = ['Q' as u8];
    match port.write(&buf) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("error writing packet: {:?}", e)),
    };

}
// Send `path` to the calculator with Conn4x-style XModem.
pub fn send_file_conn4x(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>, finish: &bool) {
    let file_contents = crate::helpers::get_file_contents(path);
    
    let packet_list = data_to_conn4x_packets(&file_contents);

    match port.write(&create_command_packet(path.file_name().unwrap(), 'P')) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("error writing packet: {:?}", e)),
    };
    
    wait_for_char(port, ACK);
    
    // XModem Server sends D to indicate that it's ready for a
    // Conn4x-style XModem transfer
    wait_for_char(port, 'D' as u8);
    
    // Now send packet_list to the serialport
    send_packets(&packet_list, port);
    if *finish {
	finish_server(port);
    }
    
}

pub fn send_file_normal(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>) {
    let file_contents = crate::helpers::get_file_contents(path);
    
    wait_for_char(port, NAK);
    
    let packet_list = data_to_128_packets(&file_contents, 0, ChecksumMode::Normal);
    //println!("{:?}", &packet_list[0..256]);
    send_packets(&packet_list, port);

}



// This function creates a "command packet" for the XModem server. It
// also adds the initial command to the entire Vec<u8>. See my XModem
// server documentation for more information about this.
fn create_command_packet(data: &OsStr, cmd: char) -> Vec<u8> {
    let mut cmd_packet: Vec<u8> = Vec::new();
    cmd_packet.push(cmd as u8);
    cmd_packet.push(((data.len() as u32 & 0xff00u32) >> 8) as u8);
    cmd_packet.push(data.len() as u8);
    let mut checksum = 0u32;
    // You can't iterate over an OsStr because it might contain
    // different encodings, so you have to convert it. You can iterate
    // over a &str, which to_str() creates.
    for c in data.to_str().unwrap().chars() {
	cmd_packet.push(c as u8);
	checksum += c as u32;
    }

    cmd_packet.push(checksum as u8);
    
    return cmd_packet;
}


// 128-byte packets sent with XModem Server are 132 bytes, hence the
// function name. The server always sends 128-byte packets even if the
// file is big enough for 1K XModem.

// TODO: this needs to remote the extra bytes at the end of the
// file. I think we can do this by looking for zeros that stretch to
// the end of the packet, in the last packet

pub fn get_file(path: &PathBuf, port: &mut Box<dyn serialport::SerialPort>, direct: &bool, overwrite: &bool, finish: &bool) {
    let mut file = match overwrite {
	true => File::create(path).unwrap(),
	false => {
	    let mut counter = 0;
	    // We loop starting with the counter at 0, until we find a
	    // file that doesn't exist. This is a bit of a hack,
	    // because we convert path to a String and then make a
	    // Path back from a modified string.
	    loop {
		let new_string = match counter {
		    0 => String::from(path.to_str().unwrap()),
		    _ => format!("{}.{:?}", path.to_str().unwrap(), counter),
		};
		let new_path = Path::new(&new_string);
		if !new_path.exists() {
		    break File::create(new_path).unwrap();
		}

		counter += 1;
	    }
	}
    };

    // We'll push to a Vec<u8>, then write to the file.
    let mut file_contents: Vec<u8> = Vec::new();

    if !direct {
	// Tell XModem server to send file
	match port.write(&create_command_packet(path.file_name().unwrap(), 'G')) {
	    Ok(_) => {},
	    Err(e) => crate::helpers::error_handler(format!("Error: failed to write packet writing packet {:?}", e)),
	}
	
	// Wait for ACK from server about command
	if wait_for_char(port, ACK) != ACK {
	    crate::helpers::error_handler("Error: got NAK from server when sending 'get' command.".to_string());
	}
	println!("got ACK");
    }

    // This is needed, probably because the calculator is pretty slow.
    thread::sleep(Duration::from_millis(500));
    // Initiate first packet from calculator by sending NAK
    let mut byte_buf: [u8; 1] = [NAK];

    match port.write(&byte_buf) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write initial NAK: {:?}", e)),
    }
    
    let mut packet_buf = vec![0; 132];
    let mut packet_counter = 0u32;
    
    loop {
	// Also needed, as far as I can tell.
	thread::sleep(Duration::from_millis(300));
	match port.read(packet_buf.as_mut_slice()) {
	    Ok(_) => {},
	    Err(e) => crate::helpers::error_handler(format!("error reading packet: {:?}", e)),
	};

	if packet_buf[0] == EOT {
	    byte_buf = [ACK];
	    match port.write(&byte_buf) {
		Ok(_) => {},
		Err(e) => crate::helpers::error_handler(format!("Error: failed to write ACK for EOT: {:?}", e)),
	    }
	    // transmission finished
	    break;
	} else if packet_buf[0] == CAN {
	    println!("Received cancel from remote side, exiting.");
	    return;
	}
	
	// verify checksum of this packet
	let mut checksum = 0u32;
	for i in &packet_buf[3..131] {
	    checksum += *i as u32;
	}

	//println!("calculated checksum is {:#x}, packet checksum is {:#x}", checksum as u8, packet_buf[131]);
	if checksum as u8 == packet_buf[131] {
	    byte_buf = [ACK];
	    // put this here instead of in the initial packet_buf
	    // read, because we only actually get a packet when the
	    // checksum matches and it's not an EOT.
	    println!("read packet {:?}", packet_counter);
	    match port.write(&byte_buf) {
		Ok(_) => {},
		Err(e) => crate::helpers::error_handler(format!(
		    "Error: failed to write ACK for packet {:?}: {:?}", packet_counter, e)),
	    }
	    file_contents.extend_from_slice(&packet_buf[3..131]);
	} else {
	    // currently untested...
	    eprintln!("Checksum failed for packet {:?}, sending NAK and trying again.", packet_counter);
	    byte_buf = [NAK];
	    match port.write(&byte_buf) {
		Ok(_) => {},
		Err(e) => crate::helpers::error_handler(format!(
		    "Error: failed to write NAK for packet {:?}: {:?}", packet_counter, e)),
	    }
	    continue; // skip packet counter increment
	}

	packet_counter += 1;
    }

    // we need to iterate backwards over file_contents and remove
    // bytes until we get a byte that isn't 0x00. Those 0x00 bytes
    // have to be consecutive, which is why we keep track of
    // last_index.
    let mut final_zero = 0;
    // no way this clone is be efficient, we should find a better way
    for (pos, c) in file_contents.clone().iter().rev().enumerate() {
	let index = file_contents.len() - 1 - pos;
	//println!("{last_index}, {index}");
	if *c != 0 {
	    final_zero = index;
	    println!("found non-zero at {index}");
	    break;
	}
    }
    // Now delete from final_zero to the end. This looks like weird
    // syntax, but if we try to delete the value of the iterator,
    // we'll outrun the vec. By deleting the same index, we delete the
    // zeros as the end of the array decreases and approaches index
    // final_zero.
    for _ in final_zero..file_contents.len() {
	file_contents.remove(final_zero);
    }
    
    match file.write_all(&file_contents) {
	Ok(_) => {},
	Err(e) => crate::helpers::error_handler(format!("Error: failed to write to output file: {:?}", e)),
    }

    if *finish {
	finish_server(port);
    }
    
}
