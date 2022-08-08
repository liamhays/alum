use std::path::PathBuf;
use std::fmt;

use console::style;

//*** Code for converting a Vec of nibbles to text
/*let mut ascix_text: Vec<char> = Vec::new();
println!("ascix_char_len is {ascix_char_len}, ascix_region_len is {ascix_region_len}");
for i in (2..2 + ascix_char_len as usize * 2).step_by(2) {
let mut b = nibs[i+1];
b <<= 4;
b |= nibs[i];
ascix_text.push(b as char);
    }
println!("ascix_text is {:?}", ascix_text);*/

fn calc_crc(crc: &mut u32, nibble: u8) {
    *crc = (*crc >> 4) ^ (((*crc ^ nibble as u32) & 0xFu32) * 0x1081u32);
}
#[derive(Debug)]
enum LengthState {
    SizeNext,
    ASCICNext,
    DirNext,
    FindEndMarker,
    Fixed,
}

pub struct ObjectInfo {
    pub romrev: char,
    pub crc: std::string::String,
    pub length: u32,
}

impl fmt::Display for ObjectInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	write!(f, "ROM Revision: {}, Object CRC: {}, Object length (bytes): {:?}",
	       style(self.romrev).green().bright(),
	       // ROM revision is not part of BYTES, so why not make
	       // it a separate color?
	       style(&self.crc).blue().bright(),
	       style(self.length as f32 / 2.0).blue().bright())
    }
}
// I am currently tempted to make this return a Result, but I don't think we need to.
fn prolog_to_length(prolog: u32) -> Option<LengthState> {
    //println!("prolog is {:x?}", prolog);
    //        DOBINT  DOREAL  DOEREL  DOCMP   DOECMP  DOCHAR  DOROMP
    for i in [0x2911, 0x2933, 0x2955, 0x2977, 0x299d, 0x29bf, 0x29e2] {
	if prolog == i {
	    return Some(LengthState::Fixed);
	}
    }
    //        DOARRY DOLNKARRY DOCSTR  DOHSTR  DOGROB  DOLIB   DOBAK   DOEXT0  DOCODE
    for i in [0x29e8, 0x2a0a,  0x2a2c, 0x2a4e, 0x2b1e, 0x2b40, 0x2b62, 0x2b88, 0x2dcc] {
	if prolog == i {
	    return Some(LengthState::SizeNext);
	}
    }

    // I'm not sure how you're supposed to get a tagged
    // object---saving one to a variable and transferring to a
    // computer didn't work for me.
    //        DOIDNT  DOLAM   DOTAG
    for i in [0x2e48, 0x2e6d, 0x2afc] {
	if prolog == i {
	    return Some(LengthState::ASCICNext);
	}
    }

    //        unit    program algebraic
    //        DOEXT   DOCOL   DOSYMB  DOLIST
    for i in [0x2ada, 0x2d9d, 0x2ab8, 0x2a74] {
	if prolog == i {
	    return Some(LengthState::FindEndMarker);
	}
    }
    //           DORRP (directory, RAM/ROM pointer)
    if prolog == 0x2a96 {
	return Some(LengthState::DirNext);
    }

    return None;
}

/**** Each prolog decoder returns a size including the prolog. ****/

// This does not need to have Option because prolog_to_length already checks for all these prologs.
fn prolog_to_fixed_length(prolog: u32) -> Result<u32, &'static str> {
    //println!("prolog to fixed length");
    match prolog {
	// DOBINT
	0x2911 => Ok(10),
	// DOREAL
	0x2933 => Ok(21),
	// DOEREL
	0x2955 => Ok(26),
	// DOCMP
	0x2977 => Ok(37),
	// DOECMP
	0x299d => Ok(47),
	// DOCHAR
	0x29bf => Ok(7),
	// DOROMP
	0x2e92 => Ok(11),
	// should never happen
	_ => Err("unknown prolog of fixed length object, this error should never happen"),
    }
}
	    
fn read_size(nibs: &Vec<u8>) -> Result<u32, &'static str> {
    // We have to go at least 10 nibbles in; if the object is less
    // than that, something is wrong.
    if nibs.len() < 10 {
	return Err("object is less than 10 nibbles long");
    }
    
    let mut length = 0u32;
    for i in (5..10).rev() {
	length <<= 4;
	length |= nibs[i] as u32;
    }
    //println!("object is {:x?}", &nibs[0..length as usize + 5]);
    // Must include prolog nibbles in this checksum
    return Ok(length + 5u32);
}

fn get_prolog(nibs: &Vec<u8>) -> Result<u32, &'static str> {//Option<u32> {
    if nibs.len() < 5 {
	return Err("object is less than 5 nibbles long");
    }
    
    let mut prolog = 0u32;
    for i in (0..5).rev() {
	prolog <<= 4;
	prolog |= nibs[i] as u32;
    }
    return Ok(prolog);
}

fn calc_object_size(nibs: &Vec<u8>) -> Result<u32, &'static str> {
    let prolog = match get_prolog(&nibs) {
	Ok(p) => p,
	Err(e) => return Err(e),
    };
    let object_length_type = prolog_to_length(prolog);
    if object_length_type.is_none() {
	return Err("unknown prolog");
    } else {
	// This length includes the prolog.  The compiler won't let us
	// use the ? operator in any of these match arms, but we can
	// check the value of the final result and Ok() it (yes, you
	// Ok() the result of ?, even if it ends up as an Err).
	
	Ok(match object_length_type {
	    Some(LengthState::SizeNext) => read_size(&nibs),
	    Some(LengthState::ASCICNext) => read_ascic_size(&nibs),
	    Some(LengthState::DirNext) => read_dir_size(&nibs),
	    Some(LengthState::Fixed) => prolog_to_fixed_length(prolog),
	    Some(LengthState::FindEndMarker) => read_size_to_end_marker(&nibs),
	    None => Err("unknown object prolog, could not calculate object length"),
	}?)
    }
}

fn read_ascic_size(nibs: &Vec<u8>) -> Result<u32, &'static str> {
    println!("read ascic size");
    // ASCIC size is encoded as a byte (so up to 255 characters). We
    // then need to go get more size, by reading the object that
    // follows the ASCIC data.
    let ascic_char_len = (nibs[1] << 4) + nibs[0];
    let ascic_region_len = 2 + ascic_char_len * 2; // nibbles
    // slice then reconvert to Vec
    let inner_nibbles = nibs[ascic_region_len as usize..].to_vec();

    let inner_region_len = calc_object_size(&inner_nibbles);
    match inner_region_len {
	Ok(inner) => return Ok(inner + ascic_region_len as u32),
	Err(e) => {
	    // so if we declare a variable, we avoid a temporary value
	    // error, but if we try to do this inline (String::from +
	    // &e), it fails to compile. odd.
	    let mut err = String::from("unable to read size of object in ASCIC field: ");
	    err.push_str(e);
	    return Err(e);
	},
    }
}

fn read_ascix_size(nibs: &Vec<u8>) -> Result<u32, &'static str> {
    //println!("read_ascix_size, nibs is {:x?}, nibs.len() is {:?}", nibs, nibs.len());
    // ASCIX consists of <1 byte length, ASCII data, same 1 byte
    // length>. It's almost identical to ASCIC.

    
    let ascix_char_len = (nibs[1] << 4) + nibs[0];
    let ascix_region_len = 2 + (ascix_char_len*2) + 2;

    
    // slice then reconvert to Vec
    let inner_nibbles = nibs[ascix_region_len as usize..].to_vec();
    //println!("{:x?}", inner_nibbles);
    let inner_region = calc_object_size(&inner_nibbles);
    match inner_region {
	Ok(inner) => Ok(inner + ascix_region_len as u32),
	// TODO: fix this to use the error in e
	Err(e) => {
	    let mut err = String::from("unable to read size of object in ASCIC field: ");
	    err.push_str(e);
	    return Err(e);
	},
    }
    //println!("inner_region is {:?} nibbles, {:?} bytes", inner_region.unwrap(), inner_region.unwrap() / 2);
}


fn read_size_to_end_marker(nibs: &Vec<u8>) -> Result<u32, &'static str> {//Option<u32> {
    //println!("read_size_to_end_marker, nibs is {:x?}", nibs);
    let mut mem_addr = 0u32; // address in Saturn memory, 5 nibbles
    for (pos, i) in nibs.iter().enumerate() {
	mem_addr <<= 4;
	mem_addr |= *i as u32;
	mem_addr &= 0xfffffu32; // Saturn uses 20-bit address
	//println!("{:?}: {:#x}", pos, mem_addr);
	
	// object end marker, reversed (SEMI is actually 0x312b)
	// because the calculator reads nibbles in reverse
	
	// note that end marker is just SEMI---so a program could
	// contain multiple secondaries, and we have to pick up
	// only the very last SEMI. the `pos == ...` term does
	// that.

	// Also, because the HP pads to the nearest byte, there could
	// actually be another 0 nibble after 'b2130', hence `...len()
	// - 2`.
	if mem_addr == 0xb2130 && (pos == nibs.len() - 1 || pos == nibs.len() - 2) {
	    //println!("found end marker, exiting");
	    
	    // add 1 to convert index to length.
	    return Ok(pos as u32 + 1);
	}
    }
    return Err("no end marker (0x0312B) found");
}


// This is a function for a specific type of variable, so 
fn read_dir_size(nibs: &Vec<u8>) -> Result<u32, &'static str> {//Option<u32> {
    //println!("read_dir_size");
    // A directory consists of the prolog (5 nibbles), attached
    // libraries (3 nibbles), an offset number (5 nibbles), and
    // 0x00000 (5 nibbles) indicating the end of the directory. The
    // calculator then reads the directory from end to beginning,
    // looking for 0x00000. We simply have to jump to the first object
    // and iterate over every object we find.

    // 5 + 3 + 5 + 5 = 18 nibbles in
    let mut index = 18usize;

    // At 18 nibbles in, the first object is defined with an ASCIX
    // name followed by the contents of the object. Every following
    // object is also an ASCIX name followed by the object's contents.
    while index < nibs.len() - 18 {
	let ascix_size = read_ascix_size(&nibs[index..].to_vec());
	match ascix_size {
	    Ok(size) => {
		index += size as usize;
		index += 5; // 5 nibble offset value after each object
	    },
	    Err(e) => return Err(e),
	}
	//println!("  ascix_size: {:?}", ascix_size);
    }

    // Subtract 5 nibbles, because the very last object in the
    // directory has no offset.
    
    // Directory objects don't include object counts, so this is
    // really the best way to do this.
    //println!("index before return is {:?}", index);
    return Ok(index as u32 - 5);
}

// A real number (and possibly other types) gives different checksums
// on the 48 and the 50, even though they are the same length on the
// calculator. The files themselves differ by one nibble at the
// end---I don't know why---but I think HP 49 checksums should not be
// printed, as they are very likely to be incorrect.

// Returns an Option enclosing an ObjectInfo struct (see above).

// This function calls external functions, which use the prolog of the
// object and parse the object to find the number of nibbles that the
// object occupies. This function makes a list of nibbles of the data,
// then uses that value to iterate over the appropriate portion of the
// file, calculating the CRC on each nibble.

fn crc_file(path: &PathBuf) -> Result<ObjectInfo, &'static str> {
    // can't use ? operator here because the function returns ObjectInfo
    let file_contents = match std::fs::read(path) {
	Err(e) => {
	    crate::helpers::error_handler(format!("Error: couldn't read file: {:?}", e));
	    Vec::new()
	},
	Ok(bytes) => bytes,
    };

    // shortest possible object is a char at 7 nibbles; 7 nibbles plus 8 bytes = 12 bytes rounded up.
    if file_contents.len() < 12 {
	return Err("file is corrupt (too short to be an HP object).");
    }
    
    let romrev_header = &file_contents[0..6];

    if romrev_header != b"HPHP48" {
	// We refuse to parse HP 49 objects because they are likely to
	// produce incorrect values.
	return Err("file is not an HP 48 binary object (does not start with HPHP48).");
    }

    let romrev = *&file_contents[7] as char;

    // split file_contents into bytes each containing one nibble
    let mut nibbles: Vec<u8> = Vec::new();
    for byte in &file_contents[8..] {
	nibbles.push(byte & 0xfu8); // low nibble
	nibbles.push(byte >> 4); // high nibble
    }

    let prolog = match get_prolog(&nibbles) {
	Ok(pro) => pro,
	Err(e) => return Err(e),
    };

    let object_length = match prolog_to_length(prolog) {
	Some(LengthState::SizeNext) => read_size(&nibbles),
	Some(LengthState::ASCICNext) => read_ascic_size(&nibbles),
	Some(LengthState::DirNext) => read_dir_size(&nibbles),
	Some(LengthState::Fixed) => prolog_to_fixed_length(prolog),
	Some(LengthState::FindEndMarker) => read_size_to_end_marker(&nibbles),
	None => return Err("unknown object prolog, could not calculate object length"),
    }?;

    
    //println!("prolog is {:?}", prolog_to_length(prolog));
    //println!("object_length is {:?}", object_length);
    // The HP 48 will expand a file send to the computer into bytes,
    // so an object that is an odd number of nibbles (like a real,
    // which is 21 nibbles), will be expanded to 22 nibbles on the
    // computer. Therefore, the calculating program must act depending
    // on the prolog of the object.

    // We have the actual number of nibbles the object occupies in
    // object_length, so we can iterate from the start to that many
    // nibbles.
    let mut crc = 0u32;
    //println!("nibble length is {:?}, nibs.len() is {:?}", object_length.unwrap(), nibbles.len());
    //println!("nibbles is {:x?}, nibs.len() is {:?}", nibbles, nibbles.len());
    if (object_length as usize) > nibbles.len() {
	return Err("object length is greater than file size; file may be corrupt");
    }
    for nibble in &nibbles[0..object_length as usize] {
	//println!("nibble is {:x?}", *nibble);
	
	// A CRC calculation sets the value of the crc variable based
	// on its previous value, therefore, we can use a mut
	// reference.
	calc_crc(&mut crc, *nibble);
    }

    // HP hex strings are uppercase
    let initial_str = format!("{:#x}", crc).to_uppercase();

    return Ok(ObjectInfo {
	romrev: romrev,
	crc: format!("#{}h", &initial_str[2..]),
	length: object_length,
    });
}


pub fn crc_and_output(path: &PathBuf) {
    let object_info = crc_file(path);
    match object_info {
	Ok(info) => println!("{}", info),
	Err(e) => crate::helpers::error_handler(format!("Error: {}", e)),
    }
}
